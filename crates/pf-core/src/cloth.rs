//! Cloth for the 3D fit preview (PF-10.3): pattern pieces at their true size made into particles
//! and links, sewn together along their sewn seams, and draped by gravity over the avatar.
//!
//! The simulation is position-based dynamics (Müller, Heidelberger, Hennix and Ratcliff,
//! "Position Based Dynamics", VRIPHYS 2006, <https://matthias-research.github.io/pages/publications/posBasedDyn.pdf>):
//! each step moves every particle by its velocity and gravity, then corrects the positions a few
//! times over so every link is as long as it was cut (stretch, shear and, across two cells,
//! bending), sewn particles meet, and no particle is inside the body. A piece is a grid of
//! particles inside its seam line, `step` apart (the smaller the step, the closer the cloth
//! follows the outline, and the longer it takes). All lengths are in mm, z up.

use crate::Pt;
use crate::avatar::{Avatar, Ring};

/// Gravity, mm/s².
pub const GRAVITY: f64 = 9_810.0;
/// Most particles in one cloth: a bound on memory and time.
pub const MAX_PARTICLES: usize = 200_000;

/// A flat piece made into particles: their places on the pattern (mm), the grid triangles between
/// them, and which particle is at each grid cell.
#[derive(Clone, Debug, PartialEq)]
pub struct Panel {
    pub points: Vec<Pt>,
    pub triangles: Vec<[u32; 3]>,
    /// Grid columns and rows, and each cell's particle (`None` outside the outline).
    cols: usize,
    rows: usize,
    grid: Vec<Option<u32>>,
    origin: Pt,
    step: f64,
}

/// Why cloth can't be made.
#[derive(Clone, Debug, PartialEq)]
pub enum ClothError {
    /// The step is not more than 0, or the outline is no polygon.
    Outline,
    /// More than [`MAX_PARTICLES`] particles: a coarser step is needed.
    TooMany,
}

impl std::fmt::Display for ClothError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            ClothError::Outline => "the piece has no outline to make cloth of",
            ClothError::TooMany => "too much cloth at this precision: give a coarser one",
        })
    }
}

impl std::error::Error for ClothError {}

/// Whether `q` is inside polygon `pts` (even-odd).
fn inside(pts: &[Pt], q: Pt) -> bool {
    let n = pts.len();
    let mut c = false;
    for i in 0..n {
        let (a, b) = (pts[i], pts[(i + 1) % n]);
        if (a.y > q.y) != (b.y > q.y) && q.x < a.x + (q.y - a.y) / (b.y - a.y) * (b.x - a.x) {
            c = !c;
        }
    }
    c
}

/// The panel of `outline` (a polygon, mm): a particle at every grid point `step` apart inside it.
pub fn panel(outline: &[Pt], step: f64) -> Result<Panel, ClothError> {
    if !(step.is_finite() && step > 0.0) || outline.len() < 3 || !outline.iter().all(|p| p.is_finite()) {
        return Err(ClothError::Outline);
    }
    let (lo, hi) = outline.iter().fold((Pt::new(f64::INFINITY, f64::INFINITY), Pt::new(f64::NEG_INFINITY, f64::NEG_INFINITY)), |(l, h), p| {
        (Pt::new(l.x.min(p.x), l.y.min(p.y)), Pt::new(h.x.max(p.x), h.y.max(p.y)))
    });
    let cols = ((hi.x - lo.x) / step).floor() as usize + 1;
    let rows = ((hi.y - lo.y) / step).floor() as usize + 1;
    if cols.saturating_mul(rows) > MAX_PARTICLES * 4 {
        return Err(ClothError::TooMany);
    }
    // Grid points just inside the edge count: a hair in, so an edge on a grid line keeps its row.
    let eps = step * 1e-6;
    let mut grid = vec![None; cols * rows];
    let mut points = Vec::new();
    for r in 0..rows {
        for c in 0..cols {
            let q = Pt::new(lo.x + c as f64 * step, lo.y + r as f64 * step);
            let nudged =
                [q, Pt::new(q.x + eps, q.y + eps), Pt::new(q.x - eps, q.y - eps), Pt::new(q.x + eps, q.y - eps), Pt::new(q.x - eps, q.y + eps)];
            if nudged.iter().any(|p| inside(outline, *p)) {
                grid[r * cols + c] = Some(points.len() as u32);
                points.push(q);
            }
        }
    }
    if points.len() > MAX_PARTICLES {
        return Err(ClothError::TooMany);
    }
    if points.len() < 3 {
        return Err(ClothError::Outline);
    }
    let at = |c: usize, r: usize| grid[r * cols + c];
    let mut triangles = Vec::new();
    for r in 0..rows.saturating_sub(1) {
        for c in 0..cols.saturating_sub(1) {
            if let (Some(a), Some(b), Some(d), Some(e)) = (at(c, r), at(c + 1, r), at(c, r + 1), at(c + 1, r + 1)) {
                triangles.push([a, b, e]);
                triangles.push([a, e, d]);
            }
        }
    }
    Ok(Panel { points, triangles, cols, rows, grid, origin: lo, step })
}

impl Panel {
    /// The particle nearest `q` on the pattern.
    pub fn nearest(&self, q: Pt) -> Option<u32> {
        let c = ((q.x - self.origin.x) / self.step).round().clamp(0.0, (self.cols - 1) as f64) as usize;
        let r = ((q.y - self.origin.y) / self.step).round().clamp(0.0, (self.rows - 1) as f64) as usize;
        if let Some(i) = self.grid[r * self.cols + c] {
            return Some(i);
        }
        // Off the grid's inside (past the edge): the nearest particle there is.
        (0..self.points.len() as u32).min_by(|a, b| {
            let d = |i: &u32| (self.points[*i as usize] - q).len();
            d(a).total_cmp(&d(b))
        })
    }

    /// The links between particles: grid neighbours (stretch), cell diagonals (shear) and
    /// neighbours but one (bending), each with its length on the pattern.
    fn links(&self) -> Vec<(u32, u32, f64, Kind)> {
        let mut out = Vec::new();
        let at = |c: usize, r: usize| if c < self.cols && r < self.rows { self.grid[r * self.cols + c] } else { None };
        for r in 0..self.rows {
            for c in 0..self.cols {
                let Some(a) = at(c, r) else { continue };
                for (dc, dr, kind) in [(1, 0, Kind::Stretch), (0, 1, Kind::Stretch), (1, 1, Kind::Shear), (2, 0, Kind::Bend), (0, 2, Kind::Bend)] {
                    if let Some(b) = at(c + dc, r + dr) {
                        out.push((a, b, (self.points[b as usize] - self.points[a as usize]).len(), kind));
                    }
                }
                // The other diagonal.
                if c > 0
                    && let Some(b) = at(c - 1, r + 1)
                {
                    out.push((a, b, (self.points[b as usize] - self.points[a as usize]).len(), Kind::Shear));
                }
            }
        }
        out
    }
}

/// What a link resists.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Stretch,
    Shear,
    Bend,
}

/// How stiff the cloth is (each from 0, giving way, to 1, holding its length every iteration).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Stiffness {
    pub stretch: f64,
    pub shear: f64,
    pub bend: f64,
}

impl Default for Stiffness {
    /// A light woven cloth: it hardly stretches, shears a little, bends easily.
    fn default() -> Self {
        Stiffness { stretch: 1.0, shear: 0.6, bend: 0.05 }
    }
}

/// What the cloth can't go into: it moves a point inside it out to its surface.
pub trait Collider {
    fn push_out(&self, p: [f64; 3]) -> [f64; 3];
}

/// The ellipse of rings `a` and `b` at height `z` between them (by straight lines).
fn between(a: &Ring, b: &Ring, z: f64) -> Option<(f64, f64, f64)> {
    let (top, bottom) = if a.z >= b.z { (a, b) } else { (b, a) };
    if z > top.z || z < bottom.z || top.z - bottom.z < 1e-9 {
        return None;
    }
    let t = (top.z - z) / (top.z - bottom.z);
    let (ta, tb) = top.semi_axes();
    let (ba, bb) = bottom.semi_axes();
    Some((top.x + (bottom.x - top.x) * t, ta + (ba - ta) * t, tb + (bb - tb) * t))
}

/// `p` moved out of the ellipse centred at `x` across with semi-axes `a` and `b` (grown by
/// `margin`), along the line from its middle.
fn out_of(p: [f64; 3], x: f64, a: f64, b: f64, margin: f64) -> [f64; 3] {
    let (a, b) = (a + margin, b + margin);
    let (dx, dy) = (p[0] - x, p[1]);
    let k = (dx / a).powi(2) + (dy / b).powi(2);
    if k >= 1.0 {
        return p;
    }
    if k < 1e-12 {
        // At the very middle: out to the front.
        return [x, b, p[2]];
    }
    let s = 1.0 / k.sqrt();
    [x + dx * s, dy * s, p[2]]
}

impl Collider for Avatar {
    /// Out of the torso and either leg, as the rings between them are at the point's height, 2 mm
    /// clear (the cloth's thickness and a little air).
    fn push_out(&self, p: [f64; 3]) -> [f64; 3] {
        const MARGIN: f64 = 2.0;
        let mut q = p;
        for rings in [&self.torso, &self.legs[0], &self.legs[1]] {
            for w in rings.windows(2) {
                if let Some((x, a, b)) = between(&w[0], &w[1], q[2]) {
                    q = out_of(q, x, a, b, MARGIN);
                    break;
                }
            }
        }
        q
    }
}

/// A panel and where it goes: its points on the pattern to points in space.
pub type Placed<'a> = (&'a Panel, &'a dyn Fn(Pt) -> [f64; 3]);

/// Cloth being draped: its particles, where they were the step before, which are held still,
/// its links and the pairs of particles sewn together.
#[derive(Clone, Debug)]
pub struct Cloth {
    pub positions: Vec<[f64; 3]>,
    previous: Vec<[f64; 3]>,
    /// 0 for a particle held still (pinned), else 1.
    weight: Vec<f64>,
    links: Vec<(u32, u32, f64, f64)>,
    /// Pairs of particles sewn together.
    pub seams: Vec<(u32, u32)>,
    /// Each panel's first particle, and the triangles of all of them (into `positions`).
    pub starts: Vec<u32>,
    pub triangles: Vec<[u32; 3]>,
    /// Each particle's place on its pattern piece, and which panel it is of.
    pattern: Vec<(Pt, u32)>,
    /// Long-range attachments: a particle no further from a held one than they are apart on the
    /// pattern (Kim, Chentanez and Müller, "Long Range Attachments", SCA 2012,
    /// <https://matthias-research.github.io/pages/publications/sca2012cloth.pdf>), so cloth that
    /// hangs doesn't stretch however few the iterations.
    tethers: Vec<(u32, u32, f64)>,
}

impl Cloth {
    /// Cloth of `panels`, each placed in 3D by its function from a point on the pattern to a
    /// point in space (its links keep their lengths on the pattern, whatever the placing does).
    pub fn new(panels: &[Placed], stiffness: Stiffness) -> Cloth {
        let mut c = Cloth {
            positions: Vec::new(),
            previous: Vec::new(),
            weight: Vec::new(),
            links: Vec::new(),
            seams: Vec::new(),
            starts: Vec::new(),
            triangles: Vec::new(),
            pattern: Vec::new(),
            tethers: Vec::new(),
        };
        for (k, (panel, place)) in panels.iter().enumerate() {
            let start = c.positions.len() as u32;
            c.starts.push(start);
            c.pattern.extend(panel.points.iter().map(|p| (*p, k as u32)));
            c.positions.extend(panel.points.iter().map(|p| place(*p)));
            c.weight.extend(std::iter::repeat_n(1.0, panel.points.len()));
            for (a, b, rest, kind) in panel.links() {
                let k = match kind {
                    Kind::Stretch => stiffness.stretch,
                    Kind::Shear => stiffness.shear,
                    Kind::Bend => stiffness.bend,
                };
                c.links.push((a + start, b + start, rest, k.clamp(0.0, 1.0)));
            }
            c.triangles.extend(panel.triangles.iter().map(|t| t.map(|i| i + start)));
        }
        c.previous = c.positions.clone();
        c
    }

    /// Hold particle `i` still (or let it go).
    pub fn pin(&mut self, i: u32, held: bool) {
        let Some(w) = self.weight.get_mut(i as usize) else { return };
        *w = if held { 0.0 } else { 1.0 };
        self.tethers.retain(|t| t.0 != i);
        if held {
            let (at, panel) = self.pattern[i as usize];
            for (j, (q, pj)) in self.pattern.iter().enumerate() {
                if *pj == panel && j != i as usize {
                    self.tethers.push((i, j as u32, (*q - at).len()));
                }
            }
        }
    }

    /// Sew particle `a` to particle `b`: they are pulled together until they meet.
    pub fn sew(&mut self, a: u32, b: u32) {
        let n = self.positions.len() as u32;
        if a < n && b < n && a != b {
            self.seams.push((a, b));
        }
    }

    /// One step of `dt` seconds: gravity, then `iterations` rounds of keeping the links' lengths,
    /// pulling the seams together and pushing the cloth out of `body`.
    pub fn step(&mut self, dt: f64, iterations: usize, body: &dyn Collider) {
        // A little air resistance: the cloth settles rather than swinging for ever.
        const DAMPING: f64 = 0.98;
        let g = GRAVITY * dt * dt;
        for i in 0..self.positions.len() {
            if self.weight[i] == 0.0 {
                continue;
            }
            let (p, q) = (self.positions[i], self.previous[i]);
            self.previous[i] = p;
            self.positions[i] = [p[0] + (p[0] - q[0]) * DAMPING, p[1] + (p[1] - q[1]) * DAMPING, p[2] + (p[2] - q[2]) * DAMPING - g];
        }
        for _ in 0..iterations {
            for &(a, b, rest, k) in &self.links {
                pull(&mut self.positions, &self.weight, a, b, rest, k);
            }
            for &(a, b) in &self.seams {
                pull(&mut self.positions, &self.weight, a, b, 0.0, 0.5);
            }
            for &(held, j, most) in &self.tethers {
                let (h, p) = (self.positions[held as usize], self.positions[j as usize]);
                let d = [p[0] - h[0], p[1] - h[1], p[2] - h[2]];
                let len = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
                if len > most && len > 1e-12 && self.weight[j as usize] > 0.0 {
                    let s = most / len;
                    self.positions[j as usize] = [h[0] + d[0] * s, h[1] + d[1] * s, h[2] + d[2] * s];
                }
            }
            for i in 0..self.positions.len() {
                if self.weight[i] > 0.0 {
                    self.positions[i] = body.push_out(self.positions[i]);
                }
            }
        }
    }

    /// How much the most stretched link is longer than it was cut, as a share (0.02: 2%).
    pub fn worst_stretch(&self) -> f64 {
        self.links
            .iter()
            .filter(|l| l.2 > 0.0)
            .map(|&(a, b, rest, _)| {
                let (pa, pb) = (self.positions[a as usize], self.positions[b as usize]);
                let len = ((pb[0] - pa[0]).powi(2) + (pb[1] - pa[1]).powi(2) + (pb[2] - pa[2]).powi(2)).sqrt();
                (len - rest) / rest
            })
            .fold(0.0, f64::max)
    }

    /// The widest gap between particles sewn together, mm.
    pub fn widest_seam(&self) -> f64 {
        self.seams
            .iter()
            .map(|&(a, b)| {
                let (pa, pb) = (self.positions[a as usize], self.positions[b as usize]);
                ((pb[0] - pa[0]).powi(2) + (pb[1] - pa[1]).powi(2) + (pb[2] - pa[2]).powi(2)).sqrt()
            })
            .fold(0.0, f64::max)
    }
}

/// Move particles `a` and `b` of `positions` (as much as each one's `weight` lets it move) toward
/// being `rest` apart, by `k` of the way.
fn pull(positions: &mut [[f64; 3]], weight: &[f64], a: u32, b: u32, rest: f64, k: f64) {
    let (a, b) = (a as usize, b as usize);
    let (wa, wb) = (weight[a], weight[b]);
    if wa + wb == 0.0 {
        return;
    }
    let (pa, pb) = (positions[a], positions[b]);
    let d = [pb[0] - pa[0], pb[1] - pa[1], pb[2] - pa[2]];
    let len = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
    if len < 1e-12 {
        return;
    }
    let c = (len - rest) / len * k / (wa + wb);
    for j in 0..3 {
        positions[a][j] += d[j] * c * wa;
        positions[b][j] -= d[j] * c * wb;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square(w: f64, h: f64) -> Vec<Pt> {
        vec![Pt::new(0.0, 0.0), Pt::new(w, 0.0), Pt::new(w, h), Pt::new(0.0, h)]
    }

    /// Nothing in the way.
    struct Air;
    impl Collider for Air {
        fn push_out(&self, p: [f64; 3]) -> [f64; 3] {
            p
        }
    }

    /// An upright cylinder `r` round its axis.
    struct Post(f64);
    impl Collider for Post {
        fn push_out(&self, p: [f64; 3]) -> [f64; 3] {
            out_of(p, 0.0, self.0, self.0, 0.0)
        }
    }

    #[test]
    fn a_panel_is_a_grid_of_particles_inside_its_outline() {
        let p = panel(&square(100.0, 100.0), 10.0).unwrap();
        assert_eq!((p.points.len(), p.triangles.len()), (121, 200));
        // An L: the notch has no particles, and no triangles span it.
        let l = vec![Pt::new(0.0, 0.0), Pt::new(100.0, 0.0), Pt::new(100.0, 40.0), Pt::new(40.0, 40.0), Pt::new(40.0, 100.0), Pt::new(0.0, 100.0)];
        let p = panel(&l, 10.0).unwrap();
        assert!(p.points.iter().all(|q| q.x <= 40.0 + 1e-9 || q.y <= 40.0 + 1e-9));
        assert!(p.points.iter().any(|q| (q.x - 100.0).abs() < 1e-9 && q.y.abs() < 1e-9), "the corner is a particle");
        assert_eq!(p.nearest(Pt::new(99.0, 1.0)).map(|i| p.points[i as usize]), Some(Pt::new(100.0, 0.0)));
        assert!(panel(&l, 0.0).is_err() && panel(&l[..2], 10.0).is_err());
    }

    #[test]
    fn cloth_held_at_two_corners_sags_without_stretching() {
        let p = panel(&square(300.0, 300.0), 20.0).unwrap();
        // Laid flat, 1 m up.
        let place: &dyn Fn(Pt) -> [f64; 3] = &|q: Pt| [q.x, q.y, 1000.0];
        let mut c = Cloth::new(&[(&p, place)], Stiffness::default());
        let corners: Vec<u32> = [Pt::new(0.0, 0.0), Pt::new(300.0, 0.0)].iter().filter_map(|q| p.nearest(*q)).collect();
        for &i in &corners {
            c.pin(i, true);
        }
        for _ in 0..300 {
            c.step(1.0 / 60.0, 10, &Air);
        }
        // The held corners where they were; the far edge hanging well below them.
        for &i in &corners {
            assert_eq!(c.positions[i as usize][2], 1000.0);
        }
        let low = c.positions.iter().map(|p| p[2]).fold(f64::INFINITY, f64::min);
        assert!(low < 1000.0 - 250.0, "{low}");
        assert!(c.worst_stretch() < 0.06, "{}", c.worst_stretch());
    }

    #[test]
    fn two_panels_sewn_into_a_tube_hang_round_a_post_and_stay_outside_it() {
        // Two panels 400 wide, 300 long, wrapped half each round a circle 800 round (radius
        // 127): sewn at both sides, held at the top, round a post of radius 100.
        let (w, h) = (400.0, 300.0);
        let p = panel(&square(w, h), 20.0).unwrap();
        let r = 2.0 * w / (2.0 * std::f64::consts::PI);
        let front: &dyn Fn(Pt) -> [f64; 3] = &|q: Pt| {
            let t = q.x / w * std::f64::consts::PI;
            [r * t.cos(), r * t.sin(), 1000.0 - q.y]
        };
        let back: &dyn Fn(Pt) -> [f64; 3] = &|q: Pt| {
            let t = std::f64::consts::PI + q.x / w * std::f64::consts::PI;
            [r * t.cos(), r * t.sin(), 1000.0 - q.y]
        };
        let mut c = Cloth::new(&[(&p, front), (&p, back)], Stiffness::default());
        let n = p.points.len() as u32;
        // The front's right side to the back's left, the back's right to the front's left.
        for y in (0..=300).step_by(20) {
            let y = y as f64;
            let (fr, fl) = (p.nearest(Pt::new(w, y)).unwrap(), p.nearest(Pt::new(0.0, y)).unwrap());
            c.sew(fr, fl + n);
            c.sew(fr + n, fl);
        }
        for x in (0..=400).step_by(20) {
            let top = p.nearest(Pt::new(x as f64, 0.0)).unwrap();
            c.pin(top, true);
            c.pin(top + n, true);
        }
        let post = Post(100.0);
        for _ in 0..200 {
            c.step(1.0 / 60.0, 10, &post);
        }
        assert!(c.positions.iter().all(|p| p[0].hypot(p[1]) >= 100.0 - 1e-6), "outside the post");
        assert!(c.widest_seam() < 3.0, "{}", c.widest_seam());
        assert!(c.worst_stretch() < 0.06, "{}", c.worst_stretch());
    }

    #[test]
    fn the_avatar_pushes_cloth_out_to_its_surface() {
        let a = crate::avatar::avatar(&crate::avatar::Body {
            bust: 880.0,
            waist: 680.0,
            hip: 960.0,
            nape_to_waist: 410.0,
            waist_to_hip: 200.0,
            ..Default::default()
        })
        .unwrap();
        let waist = a.torso[2];
        let (wa, _) = waist.semi_axes();
        // A point at the waist's middle comes out to the front, 2 mm clear; one beside the waist
        // out to the side; one well clear stays.
        let q = a.push_out([0.0, 0.0, waist.z]);
        assert!(q[1] > 0.0 && q[0].abs() < 1e-9);
        let side = a.push_out([wa * 0.5, 0.0, waist.z]);
        assert!((side[0] - (wa + 2.0)).abs() < 1e-9, "{side:?}");
        assert_eq!(a.push_out([wa + 50.0, 0.0, waist.z]), [wa + 50.0, 0.0, waist.z]);
        // Below the crotch, inside a leg: out of the leg.
        let knee = a.legs[0][1];
        let k = a.push_out([knee.x, 0.0, knee.z]);
        assert!((k[1] - (knee.semi_axes().1 + 2.0)).abs() < 1e-9, "{k:?}");
    }
}
