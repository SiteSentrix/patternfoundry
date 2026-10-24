//! The drape view (PF-10.4): draped cloth and the avatar drawn as a picture, seen from the front
//! or the side at a true scale, each triangle lit by a light over the viewer's shoulder and the
//! nearer triangle in front (a depth buffer), the cloth coloured by how tight it is.
//!
//! Tight is how much a triangle of cloth is larger than it was cut (its area over its area on
//! the pattern): at or under its cut size it is blue, half again of [`TIGHT`] green, and
//! [`TIGHT`] or more red, the colours run between (the way fit software shows strain on cloth,
//! as a colour map from loose to tight).

/// Stretched this much (a share of its area) a triangle of cloth is shown red.
pub const TIGHT: f64 = 0.10;
/// Most pixels a view takes: a bound on memory.
pub const MAX_PIXELS: u64 = 16_000_000;

/// Which way the body is seen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    /// From in front of it (looking along -y): the body's left on the picture's right.
    Front,
    /// From its left side (looking along -x): its front on the picture's right.
    Left,
}

/// One thing to draw: its points (mm), its triangles and each triangle's colour (RGB).
pub struct Shape<'a> {
    pub points: &'a [[f64; 3]],
    pub triangles: &'a [[u32; 3]],
    pub colours: &'a [[u8; 3]],
}

/// A picture: `width` × `height` pixels of RGBA (straight alpha, transparent where nothing is),
/// each pixel `mm` across, its top-left corner at `left` across and `top` up (mm, in the view's
/// own across and up).
#[derive(Clone, Debug, PartialEq)]
pub struct Picture {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
    pub mm: f64,
    pub left: f64,
    pub top: f64,
}

/// Where point `p` is seen: (across, up, depth toward the viewer).
fn seen(p: [f64; 3], side: Side) -> (f64, f64, f64) {
    match side {
        Side::Front => (p[0], p[2], p[1]),
        Side::Left => (p[1], p[2], p[0]),
    }
}

/// The colour of tightness `t` (a triangle's area over its cut area, less 1).
pub fn tightness_colour(t: f64) -> [u8; 3] {
    let k = (t / TIGHT).clamp(0.0, 1.0);
    // Blue to green to red.
    let (r, g, b) = if k < 0.5 {
        let u = k * 2.0;
        (0.0, u, 1.0 - u)
    } else {
        let u = (k - 0.5) * 2.0;
        (u, 1.0 - u, 0.0)
    };
    [(r * 220.0 + 20.0) as u8, (g * 200.0 + 30.0) as u8, (b * 220.0 + 20.0) as u8]
}

/// Draw `shapes` seen from `side`, `mm` a pixel (at least a tenth of a millimetre).
pub fn draw(shapes: &[Shape], side: Side, mm: f64) -> Option<Picture> {
    if !(mm.is_finite() && mm >= 0.1) {
        return None;
    }
    let all = shapes.iter().flat_map(|s| s.points.iter().map(|p| seen(*p, side)));
    let (l, r, b, t) = all.fold((f64::INFINITY, f64::NEG_INFINITY, f64::INFINITY, f64::NEG_INFINITY), |(l, r, b, t), q| {
        (l.min(q.0), r.max(q.0), b.min(q.1), t.max(q.1))
    });
    if !(l.is_finite() && r > l && t > b) {
        return None;
    }
    let (w, h) = (((r - l) / mm).ceil() as u64 + 1, ((t - b) / mm).ceil() as u64 + 1);
    if w * h > MAX_PIXELS {
        return None;
    }
    let (w, h) = (w as usize, h as usize);
    let mut rgba = vec![0u8; w * h * 4];
    let mut depth = vec![f64::NEG_INFINITY; w * h];
    // The light: from over the viewer's left shoulder, a little above.
    let light = {
        let v: [f64; 3] = [-0.35, 0.45, 0.82];
        let n = v[0] * v[0] + v[1] * v[1] + v[2] * v[2];
        [v[0] / n.sqrt(), v[1] / n.sqrt(), v[2] / n.sqrt()]
    };
    for s in shapes {
        for (k, tri) in s.triangles.iter().enumerate() {
            let Some(c) = s.colours.get(k) else { continue };
            let v: Vec<(f64, f64, f64)> = tri.iter().filter_map(|i| s.points.get(*i as usize)).map(|p| seen(*p, side)).collect();
            let [a, bb, cc] = v[..] else { continue };
            // In the view's own across, up and toward-the-viewer: its normal, either way out (the
            // cloth has two sides).
            let (e1, e2) = ((bb.0 - a.0, bb.1 - a.1, bb.2 - a.2), (cc.0 - a.0, cc.1 - a.1, cc.2 - a.2));
            let n = (e1.1 * e2.2 - e1.2 * e2.1, e1.2 * e2.0 - e1.0 * e2.2, e1.0 * e2.1 - e1.1 * e2.0);
            let len = (n.0 * n.0 + n.1 * n.1 + n.2 * n.2).sqrt();
            if len < 1e-12 {
                continue;
            }
            let lit = ((n.0 * light[0] + n.1 * light[1] + n.2 * light[2]) / len).abs();
            let shade = 0.35 + 0.65 * lit;
            let px = |q: (f64, f64, f64)| ((q.0 - l) / mm, (t - q.1) / mm, q.2);
            let (p0, p1, p2) = (px(a), px(bb), px(cc));
            let area = (p1.0 - p0.0) * (p2.1 - p0.1) - (p2.0 - p0.0) * (p1.1 - p0.1);
            if area.abs() < 1e-12 {
                continue;
            }
            let (x0, x1) = (p0.0.min(p1.0).min(p2.0).floor().max(0.0) as usize, (p0.0.max(p1.0).max(p2.0).ceil() as usize).min(w - 1));
            let (y0, y1) = (p0.1.min(p1.1).min(p2.1).floor().max(0.0) as usize, (p0.1.max(p1.1).max(p2.1).ceil() as usize).min(h - 1));
            for y in y0..=y1 {
                for x in x0..=x1 {
                    let (fx, fy) = (x as f64 + 0.5, y as f64 + 0.5);
                    let w0 = ((p1.0 - fx) * (p2.1 - fy) - (p2.0 - fx) * (p1.1 - fy)) / area;
                    let w1 = ((p2.0 - fx) * (p0.1 - fy) - (p0.0 - fx) * (p2.1 - fy)) / area;
                    let w2 = 1.0 - w0 - w1;
                    if w0 < 0.0 || w1 < 0.0 || w2 < 0.0 {
                        continue;
                    }
                    let z = w0 * p0.2 + w1 * p1.2 + w2 * p2.2;
                    let at = y * w + x;
                    if z <= depth[at] {
                        continue;
                    }
                    depth[at] = z;
                    for (j, ch) in c.iter().enumerate() {
                        rgba[at * 4 + j] = (*ch as f64 * shade).round().clamp(0.0, 255.0) as u8;
                    }
                    rgba[at * 4 + 3] = 255;
                }
            }
        }
    }
    Some(Picture { width: w as u32, height: h as u32, rgba, mm, left: l, top: t })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A unit square of two triangles at depth `d`, `size` mm, in the x-z plane (facing front).
    fn square(size: f64, d: f64) -> (Vec<[f64; 3]>, Vec<[u32; 3]>) {
        (vec![[0.0, d, 0.0], [size, d, 0.0], [size, d, size], [0.0, d, size]], vec![[0, 1, 2], [0, 2, 3]])
    }

    #[test]
    fn a_square_is_drawn_at_its_true_size_and_the_nearer_in_front() {
        let (p, t) = square(100.0, 0.0);
        let (q, u) = square(100.0, 10.0);
        let red = [[200, 0, 0]; 2];
        let blue = [[0, 0, 200]; 2];
        let pic = draw(&[Shape { points: &p, triangles: &t, colours: &red }, Shape { points: &q, triangles: &u, colours: &blue }], Side::Front, 1.0)
            .unwrap();
        // 100 mm at 1 mm a pixel: 101 pixels across and up.
        assert_eq!((pic.width, pic.height), (101, 101));
        // The middle pixel is the nearer square's (blue, at depth 10 toward the viewer), lit.
        let mid = ((50 * pic.width + 50) * 4) as usize;
        let px = &pic.rgba[mid..mid + 4];
        assert!(px[2] > 60 && px[0] == 0 && px[3] == 255, "{px:?}");
        // Seen from the side, a square in the x-z plane is a line: nothing much drawn.
        let side = draw(&[Shape { points: &p, triangles: &t, colours: &red }], Side::Left, 1.0);
        assert!(side.is_none_or(|s| s.rgba.chunks(4).filter(|c| c[3] == 255).count() < 200));
        assert!(draw(&[Shape { points: &p, triangles: &t, colours: &red }], Side::Front, 0.0).is_none());
    }

    #[test]
    fn tightness_runs_from_blue_to_red() {
        let (loose, mid, tight) = (tightness_colour(-0.05), tightness_colour(TIGHT / 2.0), tightness_colour(TIGHT * 2.0));
        assert!(loose[2] > loose[0] && loose[2] > loose[1]);
        assert!(mid[1] > mid[0] && mid[1] > mid[2]);
        assert!(tight[0] > tight[1] && tight[0] > tight[2]);
    }
}
