//! Windows presentation path: render offscreen, blit with
//! `UpdateLayeredWindow` (C4).
//!
//! **Why not a swapchain.** On X11/Wayland the compositor is handed a
//! surface with `CompositeAlphaMode::PreMultiplied` and honours its alpha —
//! that is what makes the overlay see-through. Windows has no such route
//! in wgpu 24: the DX12 backend reports `composite_alpha_modes: [Opaque]`
//! and hardcodes the swapchain to `DXGI_ALPHA_MODE_IGNORE`, the GL backend
//! reports `[Opaque]` too, and Win32 Vulkan drivers only ever advertise
//! `OPAQUE`. Presenting through any of them paints the desktop black
//! behind the sprites — measured on a VM, not assumed. `DwmEnableBlurBehind`
//! (which winit already calls for `with_transparent(true)`) does not rescue
//! it either: DWM ignores per-pixel alpha for a flip-model swapchain.
//!
//! **What this does instead.** `UpdateLayeredWindow` is the Win32
//! primitive for exactly this: hand the window manager a 32-bpp
//! premultiplied-BGRA bitmap and it composites it over the desktop with
//! real per-pixel alpha. So the frame is rendered into an offscreen
//! texture, copied back into a DIB section, and blitted. The sprite
//! pipeline already blends `PREMULTIPLIED_ALPHA_BLENDING` over a
//! fully-transparent clear, so the bytes are in the layout the blit wants
//! with no conversion.
//!
//! **Cost.** One full-frame GPU→CPU copy per rendered frame (w × h × 4).
//! The render loop is already paced — a static overlay redraws on a 2 s
//! heartbeat, not at 60 Hz — so the steady-state cost of an idle overlay
//! is near zero, and it is what buys correct transparency on every Windows
//! GPU, software rasterisers included.
//!
//! **Bonus.** A layered window is hit-tested against that same alpha, so
//! clicks fall through fully-transparent pixels for free. The ⚙ corner in
//! `win_overlay` still needs its `WS_EX_TRANSPARENT` toggle, because in
//! pass-through even the *opaque* sprite pixels must not catch clicks.

use std::ptr;
use windows_sys::Win32::Foundation::{POINT, SIZE};
use windows_sys::Win32::Graphics::Gdi::{
    CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, GetDC, ReleaseDC, SelectObject,
    AC_SRC_ALPHA, AC_SRC_OVER, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, BLENDFUNCTION, DIB_RGB_COLORS,
    HBITMAP, HDC, HGDIOBJ, RGBQUAD,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetWindowLongPtrW, SetWindowLongPtrW, UpdateLayeredWindow, GWL_EXSTYLE, ULW_ALPHA,
    WS_EX_LAYERED,
};

/// `copy_texture_to_buffer` requires each row to start on a 256-byte
/// boundary, so the readback buffer is wider than the image and the rows
/// are un-padded on the way into the DIB.
const COPY_ALIGN: u32 = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;

/// The texture format the offscreen target and the DIB agree on. A DIB
/// section with `BI_RGB` at 32 bpp is B, G, R, A in memory order and holds
/// sRGB-encoded values, which is exactly `Bgra8UnormSrgb`.
pub const LAYERED_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Bgra8UnormSrgb;

/// An offscreen colour target plus the GDI objects that put it on screen.
pub struct LayeredTarget {
    hwnd: isize,
    width: u32,
    height: u32,
    texture: wgpu::Texture,
    readback: wgpu::Buffer,
    padded_bytes_per_row: u32,
    dib: Dib,
}

impl LayeredTarget {
    /// Build a target for `hwnd` at `width`×`height` physical pixels.
    pub fn new(device: &wgpu::Device, hwnd: isize, width: u32, height: u32) -> Self {
        let width = width.max(1);
        let height = height.max(1);

        // `UpdateLayeredWindow` refuses a window without WS_EX_LAYERED.
        // Set it once here rather than relying on the input-region code to
        // have run first — the two are independent seams.
        ensure_layered(hwnd);

        let (texture, readback, padded_bytes_per_row) = alloc_gpu(device, width, height);
        Self {
            hwnd,
            width,
            height,
            texture,
            readback,
            padded_bytes_per_row,
            dib: Dib::new(width, height),
        }
    }

    /// The texture this frame is rendered into. egui paints onto the same
    /// one before [`present`](Self::present), exactly as it would onto a
    /// swapchain image.
    pub fn texture(&self) -> &wgpu::Texture {
        &self.texture
    }

    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        let width = width.max(1);
        let height = height.max(1);
        if width == self.width && height == self.height {
            return;
        }
        let (texture, readback, padded) = alloc_gpu(device, width, height);
        self.texture = texture;
        self.readback = readback;
        self.padded_bytes_per_row = padded;
        self.dib = Dib::new(width, height);
        self.width = width;
        self.height = height;
    }

    /// Copy the rendered frame back and hand it to the window manager.
    ///
    /// Blocking by construction: `UpdateLayeredWindow` needs the pixels in
    /// CPU memory, so the frame can't be released until the copy lands.
    /// The wait is the readback itself, which is why this is metered under
    /// the caller's `Present` perf category like a swapchain present is.
    pub fn present(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Layered Readback"),
        });
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &self.readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(self.padded_bytes_per_row),
                    rows_per_image: Some(self.height),
                },
            },
            wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
        );
        queue.submit(std::iter::once(encoder.finish()));

        let slice = self.readback.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = tx.send(result);
        });
        device.poll(wgpu::Maintain::Wait);
        match rx.recv() {
            Ok(Ok(())) => {}
            other => {
                tracing::warn!("Layered readback map failed: {other:?}; dropping frame");
                return;
            }
        }

        {
            let mapped = slice.get_mapped_range();
            self.dib.write_rows(&mapped, self.padded_bytes_per_row);
        }
        self.readback.unmap();

        self.blit();
    }

    /// Hand the DIB to the window manager. `pptdst` is null so the window
    /// keeps the position winit gave it — this call is a repaint, not a
    /// move.
    fn blit(&self) {
        let size = SIZE {
            cx: self.width as i32,
            cy: self.height as i32,
        };
        let src = POINT { x: 0, y: 0 };
        let blend = BLENDFUNCTION {
            BlendOp: AC_SRC_OVER as u8,
            BlendFlags: 0,
            SourceConstantAlpha: 255,
            // The pixels are already premultiplied by the sprite pipeline.
            AlphaFormat: AC_SRC_ALPHA as u8,
        };
        // SAFETY: `screen` is a valid screen DC for the duration; `self.dib.dc`
        // holds the selected 32-bpp section; all pointers are to live locals.
        unsafe {
            let screen = GetDC(0);
            let ok = UpdateLayeredWindow(
                self.hwnd,
                screen,
                ptr::null(),
                &size,
                self.dib.dc,
                &src,
                0,
                &blend,
                ULW_ALPHA,
            );
            ReleaseDC(0, screen);
            if ok == 0 {
                tracing::debug!(
                    "UpdateLayeredWindow failed: {}",
                    std::io::Error::last_os_error()
                );
            }
        }
    }
}

/// Allocate the offscreen colour target and its row-aligned readback
/// buffer for one size.
fn alloc_gpu(device: &wgpu::Device, width: u32, height: u32) -> (wgpu::Texture, wgpu::Buffer, u32) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Layered Frame"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: LAYERED_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });

    let padded_bytes_per_row = (width * 4).div_ceil(COPY_ALIGN) * COPY_ALIGN;
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Layered Readback"),
        size: padded_bytes_per_row as u64 * height as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    (texture, readback, padded_bytes_per_row)
}

/// Add `WS_EX_LAYERED` if it isn't already set, leaving every other style
/// (topmost, tool window, the click-through bit) alone.
fn ensure_layered(hwnd: isize) {
    // SAFETY: a style read and a conditional write on a live HWND.
    unsafe {
        let current = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        let wanted = current | WS_EX_LAYERED as isize;
        if wanted != current {
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, wanted);
        }
    }
}

/// A top-down 32-bpp DIB section and the memory DC it is selected into —
/// the source `UpdateLayeredWindow` reads from.
struct Dib {
    dc: HDC,
    bitmap: HBITMAP,
    previous: HGDIOBJ,
    /// Start of the pixel rows. Owned by the bitmap, valid until it is
    /// deleted; rows are `width * 4` bytes with no padding.
    bits: *mut u8,
    stride: usize,
    height: u32,
}

impl Dib {
    fn new(width: u32, height: u32) -> Self {
        let header = BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width as i32,
            // Negative height = top-down rows, matching the texture's
            // origin so no vertical flip is needed on the copy.
            biHeight: -(height as i32),
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB,
            biSizeImage: 0,
            biXPelsPerMeter: 0,
            biYPelsPerMeter: 0,
            biClrUsed: 0,
            biClrImportant: 0,
        };
        let info = BITMAPINFO {
            bmiHeader: header,
            // Unused at 32 bpp with BI_RGB — there is no palette — but the
            // struct carries the array, so it gets a zeroed entry.
            bmiColors: [RGBQUAD {
                rgbBlue: 0,
                rgbGreen: 0,
                rgbRed: 0,
                rgbReserved: 0,
            }],
        };

        let mut bits: *mut std::ffi::c_void = ptr::null_mut();
        // SAFETY: `info` is a fully initialised BITMAPINFO describing a
        // 32-bpp BI_RGB section; `bits` receives the pixel pointer, which
        // stays valid until `DeleteObject` in `Drop`.
        let (dc, bitmap, previous) = unsafe {
            let screen = GetDC(0);
            let dc = CreateCompatibleDC(screen);
            let bitmap = CreateDIBSection(screen, &info, DIB_RGB_COLORS, &mut bits, 0, 0);
            ReleaseDC(0, screen);
            let previous = SelectObject(dc, bitmap);
            (dc, bitmap, previous)
        };

        Self {
            dc,
            bitmap,
            previous,
            bits: bits.cast::<u8>(),
            stride: width as usize * 4,
            height,
        }
    }

    /// Un-pad the readback rows into the bitmap.
    fn write_rows(&self, mapped: &[u8], padded_bytes_per_row: u32) {
        if self.bits.is_null() {
            return;
        }
        let padded = padded_bytes_per_row as usize;
        for row in 0..self.height as usize {
            let start = row * padded;
            let Some(src) = mapped.get(start..start + self.stride) else {
                break;
            };
            // SAFETY: `bits` points at `height * stride` bytes owned by the
            // bitmap, and `row < height`, so the destination range is in
            // bounds; source and destination never overlap.
            unsafe {
                ptr::copy_nonoverlapping(
                    src.as_ptr(),
                    self.bits.add(row * self.stride),
                    self.stride,
                )
            };
        }
    }
}

impl Drop for Dib {
    fn drop(&mut self) {
        // SAFETY: unwind the exact objects created in `new`, in reverse —
        // the bitmap has to leave the DC before either can be deleted.
        unsafe {
            SelectObject(self.dc, self.previous);
            DeleteObject(self.bitmap);
            DeleteDC(self.dc);
        }
    }
}
