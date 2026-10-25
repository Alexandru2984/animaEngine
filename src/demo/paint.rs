//! A small painter for the demo characters: each shape a signed distance
//! field — negative inside, in pixels — so its edge is anti-aliased by the
//! distance itself, an outline is the shape grown by its width, and a
//! shade can read how far in a pixel is. Light comes from the upper left.

use image::{Rgba, RgbaImage};

/// Premultiplied RGBA, 0..=1, a pixel each.
pub struct Canvas {
    size: u32,
    px: Vec<[f32; 4]>,
}

/// An RGB colour from 0..=255 components.
pub fn rgb(r: u8, g: u8, b: u8) -> [f32; 3] {
    [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0]
}

/// `c` scaled toward black (`k` < 1) or white (`k` > 1, by how much past 1).
pub fn shade(c: [f32; 3], k: f32) -> [f32; 3] {
    if k <= 1.0 {
        c.map(|v| v * k)
    } else {
        let t = (k - 1.0).min(1.0);
        c.map(|v| v + (1.0 - v) * t)
    }
}

pub fn mix(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    let t = t.clamp(0.0, 1.0);
    [0, 1, 2].map(|i| a[i] + (b[i] - a[i]) * t)
}

impl Canvas {
    pub fn new(size: u32) -> Self {
        Self {
            size,
            px: vec![[0.0; 4]; (size * size) as usize],
        }
    }

    /// Lay `color(x, y)` — RGB and alpha — where `dist(x, y) < 0`, its edge
    /// a pixel wide.
    pub fn fill(
        &mut self,
        dist: impl Fn(f32, f32) -> f32,
        color: impl Fn(f32, f32) -> ([f32; 3], f32),
    ) {
        let n = self.size;
        for y in 0..n {
            for x in 0..n {
                let (fx, fy) = (x as f32 + 0.5, y as f32 + 0.5);
                let cover = (0.5 - dist(fx, fy)).clamp(0.0, 1.0);
                if cover <= 0.0 {
                    continue;
                }
                let (c, alpha) = color(fx, fy);
                self.over((x, y), c, alpha * cover);
            }
        }
    }

    /// `fill` in one colour.
    pub fn solid(&mut self, dist: impl Fn(f32, f32) -> f32, c: [f32; 3], alpha: f32) {
        self.fill(dist, |_, _| (c, alpha));
    }

    /// A soft shadow on the ground: an ellipse fading out over its radii.
    pub fn ground_shadow(&mut self, cx: f32, cy: f32, rx: f32, ry: f32, alpha: f32) {
        let n = self.size;
        for y in 0..n {
            for x in 0..n {
                let dx = (x as f32 + 0.5 - cx) / rx;
                let dy = (y as f32 + 0.5 - cy) / ry;
                let r = (dx * dx + dy * dy).sqrt();
                if r < 1.0 {
                    let a = alpha * (1.0 - r * r);
                    self.over((x, y), [0.05, 0.03, 0.10], a);
                }
            }
        }
    }

    fn over(&mut self, (x, y): (u32, u32), c: [f32; 3], a: f32) {
        let p = &mut self.px[(y * self.size + x) as usize];
        let keep = 1.0 - a;
        for i in 0..3 {
            p[i] = c[i] * a + p[i] * keep;
        }
        p[3] = a + p[3] * keep;
    }

    pub fn into_image(self) -> RgbaImage {
        let mut img = RgbaImage::new(self.size, self.size);
        for (i, p) in self.px.iter().enumerate() {
            let a = p[3].clamp(0.0, 1.0);
            let c = |v: f32| {
                if a > 0.0 {
                    ((v / a).clamp(0.0, 1.0) * 255.0).round() as u8
                } else {
                    0
                }
            };
            let (x, y) = (i as u32 % self.size, i as u32 / self.size);
            img.put_pixel(
                x,
                y,
                Rgba([c(p[0]), c(p[1]), c(p[2]), (a * 255.0).round() as u8]),
            );
        }
        img
    }
}

// ── Shapes: distance in pixels, negative inside ─────────────────────────

pub fn circle(cx: f32, cy: f32, r: f32) -> impl Fn(f32, f32) -> f32 {
    move |x, y| ((x - cx).powi(2) + (y - cy).powi(2)).sqrt() - r
}

/// An ellipse, near enough at the edge, which is all the painting uses.
pub fn ellipse(cx: f32, cy: f32, rx: f32, ry: f32) -> impl Fn(f32, f32) -> f32 {
    move |x, y| {
        let (dx, dy) = ((x - cx) / rx, (y - cy) / ry);
        ((dx * dx + dy * dy).sqrt() - 1.0) * rx.min(ry)
    }
}

/// A segment from `a` to `b`, `r` thick on each side.
pub fn capsule(ax: f32, ay: f32, bx: f32, by: f32, r: f32) -> impl Fn(f32, f32) -> f32 {
    move |x, y| {
        let (px, py, vx, vy) = (x - ax, y - ay, bx - ax, by - ay);
        let h = ((px * vx + py * vy) / (vx * vx + vy * vy)).clamp(0.0, 1.0);
        ((px - vx * h).powi(2) + (py - vy * h).powi(2)).sqrt() - r
    }
}

/// A triangle through three points, its corners rounded by `r`.
pub fn triangle(p: [(f32, f32); 3], r: f32) -> impl Fn(f32, f32) -> f32 {
    move |x, y| {
        let mut d = f32::MAX;
        let mut inside = true;
        for i in 0..3 {
            let (a, b) = (p[i], p[(i + 1) % 3]);
            let (ex, ey) = (b.0 - a.0, b.1 - a.1);
            let (wx, wy) = (x - a.0, y - a.1);
            let h = ((wx * ex + wy * ey) / (ex * ex + ey * ey)).clamp(0.0, 1.0);
            d = d.min(((wx - ex * h).powi(2) + (wy - ey * h).powi(2)).sqrt());
            let cross = ex * wy - ey * wx;
            let orient =
                (p[1].0 - p[0].0) * (p[2].1 - p[0].1) - (p[1].1 - p[0].1) * (p[2].0 - p[0].0);
            if cross * orient < 0.0 {
                inside = false;
            }
        }
        (if inside { -d } else { d }) - r
    }
}

/// A heart, its tip at (`cx`, `tip_y`), `size` pixels from tip to top.
pub fn heart(cx: f32, tip_y: f32, size: f32) -> impl Fn(f32, f32) -> f32 {
    move |x, y| {
        // Inigo Quilez's heart, in units where it is one tall.
        let px = ((x - cx) / size).abs();
        let py = (tip_y - y) / size;
        let dot2 = |a: f32, b: f32| a * a + b * b;
        let d = if py + px > 1.0 {
            dot2(px - 0.25, py - 0.75).sqrt() - std::f32::consts::SQRT_2 / 4.0
        } else {
            let m = 0.5 * (px + py).max(0.0);
            dot2(px, py - 1.0).min(dot2(px - m, py - m)).sqrt() * (px - py).signum()
        };
        d * size
    }
}

/// A five-pointed star of outer radius `r`, inner at `r * rf`, turned
/// by `angle`, its corners rounded by `round`.
pub fn star(cx: f32, cy: f32, r: f32, rf: f32, angle: f32, round: f32) -> impl Fn(f32, f32) -> f32 {
    move |x, y| {
        // Inigo Quilez's five-pointed star; y up.
        let (s, c) = angle.sin_cos();
        let (dx, dy) = (x - cx, cy - y);
        let (mut px, mut py) = (dx * c - dy * s, dx * s + dy * c);
        let k1 = (0.809_017_f32, -0.587_785_25_f32);
        let k2 = (-k1.0, k1.1);
        px = px.abs();
        let d1 = 2.0 * (k1.0 * px + k1.1 * py).max(0.0);
        px -= d1 * k1.0;
        py -= d1 * k1.1;
        let d2 = 2.0 * (k2.0 * px + k2.1 * py).max(0.0);
        px -= d2 * k2.0;
        py -= d2 * k2.1;
        px = px.abs();
        py -= r;
        let (bax, bay) = (rf * -k1.1, rf * k1.0 - 1.0);
        let h = ((px * bax + py * bay) / (bax * bax + bay * bay)).clamp(0.0, r);
        let len = ((px - bax * h).powi(2) + (py - bay * h).powi(2)).sqrt();
        len * (py * bax - px * bay).signum() - round
    }
}

pub fn union(a: impl Fn(f32, f32) -> f32, b: impl Fn(f32, f32) -> f32) -> impl Fn(f32, f32) -> f32 {
    move |x, y| a(x, y).min(b(x, y))
}

/// A union whose seam is rounded over `k` pixels.
pub fn blend(
    a: impl Fn(f32, f32) -> f32,
    b: impl Fn(f32, f32) -> f32,
    k: f32,
) -> impl Fn(f32, f32) -> f32 {
    move |x, y| {
        let (da, db) = (a(x, y), b(x, y));
        let h = (0.5 + 0.5 * (db - da) / k).clamp(0.0, 1.0);
        db + (da - db) * h - k * h * (1.0 - h)
    }
}

/// `d` grown by `w` pixels — an outline's shape.
pub fn grow(d: impl Fn(f32, f32) -> f32, w: f32) -> impl Fn(f32, f32) -> f32 {
    move |x, y| d(x, y) - w
}

// ── Light ───────────────────────────────────────────────────────────────

/// `base` lit as a rounded body whose middle is (`cx`, `cy`) and whose
/// reach is (`rx`, `ry`): brighter up and left, darker down and right,
/// with a soft gloss.
pub fn lit(base: [f32; 3], x: f32, y: f32, cx: f32, cy: f32, rx: f32, ry: f32) -> [f32; 3] {
    let nx = ((x - cx) / rx).clamp(-1.0, 1.0);
    let ny = ((y - cy) / ry).clamp(-1.0, 1.0);
    let nz = (1.0 - nx * nx - ny * ny).max(0.0).sqrt();
    let (lx, ly, lz) = (-0.45_f32, -0.6_f32, 0.66_f32);
    let diffuse = (nx * lx + ny * ly + nz * lz).max(0.0);
    let lit = shade(base, 0.62 + 0.5 * diffuse);
    // Gloss: the half vector toward a viewer straight ahead.
    let (hx, hy, hz) = (lx, ly, lz + 1.0);
    let hl = (hx * hx + hy * hy + hz * hz).sqrt();
    let spec = ((nx * hx + ny * hy + nz * hz) / hl).max(0.0).powi(24);
    mix(lit, [1.0, 1.0, 1.0], spec * 0.55)
}

// ── Faces ───────────────────────────────────────────────────────────────

/// Two eyes — open, with a pupil and a catch light, or closed as a curve
/// — at `y`, `gap` apart, `r` across.
pub fn eyes(c: &mut Canvas, cx: f32, y: f32, gap: f32, r: f32, open: f32, ink: [f32; 3]) {
    for side in [-1.0_f32, 1.0] {
        let ex = cx + side * gap / 2.0;
        if open > 0.15 {
            let ry = r * 1.15 * open;
            c.solid(ellipse(ex, y, r, ry), ink, 1.0);
            c.solid(
                circle(ex - r * 0.3, y - ry * 0.35, r * 0.38),
                [1.0; 3],
                0.95,
            );
            c.solid(circle(ex + r * 0.3, y + ry * 0.3, r * 0.16), [1.0; 3], 0.7);
        } else {
            c.solid(capsule(ex - r, y, ex + r, y, 1.1), ink, 1.0);
        }
    }
}

/// A small smile: an arc under (`cx`, `y`), `w` wide.
pub fn smile(c: &mut Canvas, cx: f32, y: f32, w: f32, ink: [f32; 3]) {
    let arc = move |x: f32, yy: f32| {
        let r = w * 0.75;
        let d = ((x - cx).powi(2) + (yy - (y - r * 0.55)).powi(2)).sqrt() - r;
        let below = yy - (y - r * 0.15);
        d.abs().max(-below) - 1.1
    };
    c.solid(arc, ink, 1.0);
}

/// Rosy cheeks.
pub fn blush(c: &mut Canvas, cx: f32, y: f32, gap: f32, r: f32) {
    for side in [-1.0_f32, 1.0] {
        c.solid(
            ellipse(cx + side * gap / 2.0, y, r, r * 0.6),
            rgb(255, 120, 150),
            0.45,
        );
    }
}

/// A four-pointed sparkle, `r` long, at `alpha`.
pub fn sparkle(c: &mut Canvas, x: f32, y: f32, r: f32, alpha: f32) {
    if alpha <= 0.01 {
        return;
    }
    let s = union(
        capsule(x - r, y, x + r, y, 0.9),
        capsule(x, y - r, x, y + r, 0.9),
    );
    c.solid(s, [1.0, 0.98, 0.85], alpha);
    c.solid(circle(x, y, r * 0.3), [1.0; 3], alpha);
}
