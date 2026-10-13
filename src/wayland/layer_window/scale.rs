//! Hi-DPI (1.5): each surface drawn at its own output's scale.
//!
//! A layer surface's buffer was one pixel per surface unit, so on an
//! output at scale 2 the compositor stretched it to twice its size,
//! sprites and text blurred. egui meanwhile laid the panel out at the
//! largest output's scale in those same units, which the pointer did not
//! use: on a scale-2 output the panel came out twice its size and every
//! click landed at half the distance from the corner.
//!
//! Now a surface's buffer is `size × scale` pixels, shown at its size
//! through `wp_viewport.set_destination`, and `scale` is the one
//! `wp_fractional_scale_v1` names for it — 1.25, 1.5 — or, without that
//! protocol, the whole number its output has. Everything else keeps to
//! surface units: positions, the input region, the pointer, egui's
//! points. Only the swapchain's size and egui's pixels per point see the
//! scale. Without `wp_viewporter` a surface stays at scale 1, as before.

use super::state::WaylandState;
use wayland_client::{
    globals::GlobalList, protocol::wl_surface::WlSurface, Connection, Dispatch, QueueHandle,
};
use wayland_protocols::wp::fractional_scale::v1::client::{
    wp_fractional_scale_manager_v1::{self, WpFractionalScaleManagerV1},
    wp_fractional_scale_v1::{self, WpFractionalScaleV1},
};
use wayland_protocols::wp::viewporter::client::{
    wp_viewport::{self, WpViewport},
    wp_viewporter::{self, WpViewporter},
};

/// The scales a surface is drawn at, at most: past them a compositor's
/// number is taken as a mistake rather than a 32-fold buffer.
const MIN_SCALE: f32 = 0.5;
const MAX_SCALE: f32 = 8.0;

/// The compositor's globals for it, those it has.
pub struct ScaleGlobals {
    viewporter: Option<WpViewporter>,
    fractional: Option<WpFractionalScaleManagerV1>,
}

impl ScaleGlobals {
    pub fn bind(globals: &GlobalList, qh: &QueueHandle<WaylandState>) -> Self {
        Self {
            viewporter: globals.bind(qh, 1..=1, ()).ok(),
            fractional: globals.bind(qh, 1..=1, ()).ok(),
        }
    }
}

/// One surface's scale, and what it is asked through.
pub struct SurfaceScale {
    viewport: Option<WpViewport>,
    fractional: Option<WpFractionalScaleV1>,
    /// What to draw it at, as last told; 1 until then.
    scale: f32,
}

impl SurfaceScale {
    /// Nothing asked, scale 1: for a surface not made yet.
    pub fn none() -> Self {
        Self {
            viewport: None,
            fractional: None,
            scale: 1.0,
        }
    }

    pub fn new(
        globals: &ScaleGlobals,
        surface: &WlSurface,
        qh: &QueueHandle<WaylandState>,
    ) -> Self {
        let viewport = globals
            .viewporter
            .as_ref()
            .map(|v| v.get_viewport(surface, qh, ()));
        // Its scale is no use without a viewport to show the larger
        // buffer at the surface's size.
        let fractional = match (&viewport, &globals.fractional) {
            (Some(_), Some(manager)) => {
                Some(manager.get_fractional_scale(surface, qh, surface.clone()))
            }
            _ => None,
        };
        Self {
            viewport,
            fractional,
            scale: 1.0,
        }
    }

    /// What to draw the surface at.
    pub fn scale(&self) -> f32 {
        self.scale
    }

    /// The surface is `width × height` units: its buffer is shown at that
    /// size, however many pixels it has.
    pub fn set_size(&self, width: u32, height: u32) {
        if let (Some(viewport), Ok(w), Ok(h)) =
            (&self.viewport, i32::try_from(width), i32::try_from(height))
        {
            if w > 0 && h > 0 {
                viewport.set_destination(w, h);
            }
        }
    }

    /// The whole-number scale `wl_surface` reports (`scale_factor_changed`):
    /// taken when there is no fractional one to be had.
    pub fn offer_integer(&mut self, factor: i32) {
        if self.fractional.is_none() {
            self.offer(factor as f32);
        }
    }

    fn offer(&mut self, scale: f32) {
        if self.viewport.is_some() && scale.is_finite() {
            self.scale = scale.clamp(MIN_SCALE, MAX_SCALE);
        }
    }
}

impl Drop for SurfaceScale {
    fn drop(&mut self) {
        if let Some(fractional) = self.fractional.take() {
            fractional.destroy();
        }
        if let Some(viewport) = self.viewport.take() {
            viewport.destroy();
        }
    }
}

impl Dispatch<WpViewporter, ()> for WaylandState {
    fn event(
        _: &mut Self,
        _: &WpViewporter,
        _: wp_viewporter::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        // No events.
    }
}

impl Dispatch<WpViewport, ()> for WaylandState {
    fn event(
        _: &mut Self,
        _: &WpViewport,
        _: wp_viewport::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        // No events.
    }
}

impl Dispatch<WpFractionalScaleManagerV1, ()> for WaylandState {
    fn event(
        _: &mut Self,
        _: &WpFractionalScaleManagerV1,
        _: wp_fractional_scale_manager_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        // No events.
    }
}

impl Dispatch<WpFractionalScaleV1, WlSurface> for WaylandState {
    fn event(
        state: &mut Self,
        _: &WpFractionalScaleV1,
        event: wp_fractional_scale_v1::Event,
        surface: &WlSurface,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        // In 120ths: 180 is 1.5.
        if let wp_fractional_scale_v1::Event::PreferredScale { scale } = event {
            if let Some(this) = state.scale_of(surface) {
                this.offer(scale as f32 / 120.0);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// No viewport, no scale: the buffer has to stay one pixel a unit.
    #[test]
    fn without_a_viewport_the_scale_stays_one() {
        let mut s = SurfaceScale::none();
        s.offer_integer(2);
        s.offer(1.5);
        assert_eq!(s.scale(), 1.0);
    }
}
