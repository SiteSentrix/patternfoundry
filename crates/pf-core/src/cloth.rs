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

/// The point of polygon `pts`'s outline nearest `q`.
fn nearest_on(pts: &[Pt], q: Pt) -> Pt {
    let n = pts.len();
    let mut best = (f64::INFINITY, q);
    for i in 0..n {
        let (a, b) = (pts[i], pts[(i + 1) % n]);
        let d = b - a;
        let t = if d.dot(d) > 1e-18 { ((q - a).dot(d) / d.dot(d)).clamp(0.0, 1.0) } else { 0.0 };
        let p = a + d * t;
        let l = (p - q).len();
        if l < best.0 {
            best = (l, p);
        }
    }
    best.1
}

/// The panel of `outline` (a polygon, mm): a particle at every grid point `step` apart inside it,
/// those at the grid's edge moved onto the outline (where it is within a step), so the cloth
/// reaches its seam lines and a seam sews the pieces' edges, not points a step inside them.
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
    // The grid's edge: particles missing a neighbour across, along or diagonally.
    let edge = |c: usize, r: usize| {
        [(-1i64, 0i64), (1, 0), (0, -1), (0, 1), (-1, -1), (1, 1), (-1, 1), (1, -1)].iter().any(|&(dc, dr)| {
            let (cc, rr) = (c as i64 + dc, r as i64 + dr);
            cc < 0 || rr < 0 || cc >= cols as i64 || rr >= rows as i64 || grid[rr as usize * cols + cc as usize].is_none()
        })
    };
    let mut moved: Vec<Pt> = Vec::new();
    for r in 0..rows {
        for c in 0..cols {
            let Some(i) = at(c, r) else { continue };
            if !edge(c, r) {
                continue;
            }
            let q = points[i as usize];
            let to = nearest_on(outline, q);
            // Not two particles in one place (as at a corner): keep a quarter step apart.
            if (to - q).len() <= step && moved.iter().all(|m| (*m - to).len() > step * 0.25) {
                points[i as usize] = to;
                moved.push(to);
            }
        }
    }
    let mut triangles = Vec::new();
    for r in 0..rows.saturating_sub(1) {
        for c in 0..cols.saturating_sub(1) {
            match (at(c, r), at(c + 1, r), at(c, r + 1), at(c + 1, r + 1)) {
                (Some(a), Some(b), Some(d), Some(e)) => {
                    triangles.push([a, b, e]);
                    triangles.push([a, e, d]);
                }
                // A cell the outline cuts across (along a slanted side): the triangle of the three
                // corners inside, so the cloth runs on to its edge without a sawtooth of gaps.
                (Some(a), Some(b), None, Some(e)) => triangles.push([a, b, e]),
                (Some(a), None, Some(d), Some(e)) => triangles.push([a, e, d]),
                (Some(a), Some(b), Some(d), None) => triangles.push([a, b, d]),
                (None, Some(b), Some(d), Some(e)) => triangles.push([b, e, d]),
                _ => {}
            }
        }
    }
    Ok(Panel { points, triangles, cols, rows, grid, origin: lo, step })
}

impl Panel {
    /// The particle nearest `q` on the pattern (by where the particles are, the edge's moved onto
    /// the outline).
    pub fn nearest(&self, q: Pt) -> Option<u32> {
        let c = ((q.x - self.origin.x) / self.step).round().clamp(0.0, (self.cols - 1) as f64) as i64;
        let r = ((q.y - self.origin.y) / self.step).round().clamp(0.0, (self.rows - 1) as f64) as i64;
        // The cells round it first; anywhere when none of them has one.
        let near = (-1..=1)
            .flat_map(|dr| (-1..=1).map(move |dc| (c + dc, r + dr)))
            .filter(|&(cc, rr)| cc >= 0 && rr >= 0 && (cc as usize) < self.cols && (rr as usize) < self.rows)
            .filter_map(|(cc, rr)| self.grid[rr as usize * self.cols + cc as usize])
            .min_by(|a, b| (self.points[*a as usize] - q).len().total_cmp(&(self.points[*b as usize] - q).len()));
        if near.is_some() {
            return near;
        }
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

/// The ellipse of rings `a` and `b` at height `z` between them (by straight lines): its middle
/// across and toward the front, and its semi-axes.
fn between(a: &Ring, b: &Ring, z: f64) -> Option<(f64, f64, f64, f64)> {
    let (top, bottom) = if a.z >= b.z { (a, b) } else { (b, a) };
    if z > top.z || z < bottom.z || top.z - bottom.z < 1e-9 {
        return None;
    }
    let t = (top.z - z) / (top.z - bottom.z);
    let (ta, tb) = top.semi_axes();
    let (ba, bb) = bottom.semi_axes();
    Some((top.x + (bottom.x - top.x) * t, top.y + (bottom.y - top.y) * t, ta + (ba - ta) * t, tb + (bb - tb) * t))
}

/// `p` moved out of the ellipse centred at (`x`, `y`) with semi-axes `a` and `b` (grown by
/// `margin`), along the line from its middle.
fn out_of(p: [f64; 3], (x, y): (f64, f64), a: f64, b: f64, margin: f64) -> [f64; 3] {
    let (a, b) = (a + margin, b + margin);
    let (dx, dy) = (p[0] - x, p[1] - y);
    let k = (dx / a).powi(2) + (dy / b).powi(2);
    if k >= 1.0 {
        return p;
    }
    if k < 1e-12 {
        // At the very middle: out to the front.
        return [x, y + b, p[2]];
    }
    let s = 1.0 / k.sqrt();
    [x + dx * s, y + dy * s, p[2]]
}

/// How far inside the body made of `rings` (top to bottom) point `p` is: its distance from the
/// middle as a share of the way out to the surface, less 1 (`< 0` inside; nearly linear across
/// the surface, so Newton's steps on it don't overshoot), of the rings' shape at its height grown
/// by `margin`, or None above or below it.
fn inside_by(rings: &[Ring], p: [f64; 3], margin: f64) -> Option<f64> {
    rings
        .windows(2)
        .find_map(|w| between(&w[0], &w[1], p[2]))
        .map(|(x, y, a, b)| (((p[0] - x) / (a + margin)).powi(2) + ((p[1] - y) / (b + margin)).powi(2)).sqrt() - 1.0)
}

/// `p` moved out of the body made of `rings` along the surface's normal (where the body widens
/// downward, as from the waist to the hips, the normal leans up, and holds cloth up): Newton
/// steps on the ellipse function, its slope up and down taken across 1 mm.
fn out_of_body(rings: &[Ring], p: [f64; 3], margin: f64) -> [f64; 3] {
    let mut q = p;
    for _ in 0..4 {
        let Some(f) = inside_by(rings, q, margin).filter(|f| *f < 0.0) else { break };
        let h = 0.5;
        let g = |i: usize| {
            let (mut lo, mut hi) = (q, q);
            lo[i] -= h;
            hi[i] += h;
            match (inside_by(rings, lo, margin), inside_by(rings, hi, margin)) {
                (Some(a), Some(b)) => (b - a) / (2.0 * h),
                _ => 0.0,
            }
        };
        let grad = [g(0), g(1), g(2)];
        let n2 = grad[0] * grad[0] + grad[1] * grad[1] + grad[2] * grad[2];
        if n2 < 1e-18 {
            break;
        }
        // To the surface and a hair past it.
        let s = -(f - 1e-9) / n2;
        q = [q[0] + grad[0] * s, q[1] + grad[1] * s, q[2] + grad[2] * s];
    }
    // Still inside (at the very middle, or where the slope can't be taken): out sideways.
    match inside_by(rings, q, margin) {
        Some(f) if f < 0.0 => rings.windows(2).find_map(|w| between(&w[0], &w[1], q[2])).map_or(q, |(x, y, a, b)| out_of(q, (x, y), a, b, margin)),
        _ => q,
    }
}

impl Collider for Avatar {
    /// Out of the torso, either leg and either arm, along the surface as the rings shape it, 2 mm
    /// clear (the cloth's thickness and a little air).
    fn push_out(&self, p: [f64; 3]) -> [f64; 3] {
        const MARGIN: f64 = 2.0;
        // Where two parts meet (the arm and the side of the chest at the armpit) pushing out of one
        // can push into the other: again until out of both (a few times at most).
        let mut q = p;
        for _ in 0..4 {
            let before = q;
            for rings in self.chains() {
                q = out_of_body(rings, q, MARGIN);
            }
            if q == before {
                break;
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
    /// Whether each seam pulls (one sewn later is off until then).
    pub seam_on: Vec<bool>,
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
    /// Particles the body pushed this step (for friction).
    touching: Vec<bool>,
    /// How much of its sliding a particle touching the body loses each step ([`FRICTION`]; 0
    /// while a garment is being put on, so it slides into place).
    pub friction: f64,
    /// How hard the seams pull each round of corrections (0.5: they meet; less while the garment
    /// is being sewn up, so its pieces come together gently rather than all at once).
    pub seam_pull: f64,
    /// How much of gravity pulls (1; 0 while the garment is sewn up round the body, so it isn't
    /// pulled off before its seams have closed).
    pub gravity: f64,
}

/// How much of its sliding a particle touching the body loses each step (cloth on skin, or on
/// the cloth under it: it grips).
pub const FRICTION: f64 = 0.8;

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
            seam_on: Vec::new(),
            starts: Vec::new(),
            triangles: Vec::new(),
            pattern: Vec::new(),
            tethers: Vec::new(),
            touching: Vec::new(),
            friction: FRICTION,
            seam_pull: 0.5,
            gravity: 1.0,
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
        c.touching = vec![false; c.positions.len()];
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

    /// Hold particle `i` still (`held`) or let it go, without tethering the rest of its panel to
    /// it (as [`Cloth::pin`] does): while a garment is put on a piece at a time.
    pub fn hold(&mut self, i: u32, held: bool) {
        if let Some(w) = self.weight.get_mut(i as usize) {
            *w = if held { 0.0 } else { 1.0 };
        }
    }

    /// Sew particle `a` to particle `b`: they are pulled together until they meet.
    pub fn sew(&mut self, a: u32, b: u32) {
        let n = self.positions.len() as u32;
        if a < n && b < n && a != b {
            self.seams.push((a, b));
            self.seam_on.push(true);
        }
    }

    /// One step of `dt` seconds: gravity, then `iterations` rounds of keeping the links' lengths,
    /// pulling the seams together and pushing the cloth out of `body`.
    pub fn step(&mut self, dt: f64, iterations: usize, body: &dyn Collider) {
        // A little air resistance: the cloth settles rather than swinging for ever.
        const DAMPING: f64 = 0.98;
        let g = GRAVITY * self.gravity * dt * dt;
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
            for (&(a, b), _) in self.seams.iter().zip(&self.seam_on).filter(|(_, on)| **on) {
                pull(&mut self.positions, &self.weight, a, b, 0.0, self.seam_pull);
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
                    let q = body.push_out(self.positions[i]);
                    if q != self.positions[i] {
                        self.touching[i] = true;
                    }
                    self.positions[i] = q;
                }
            }
        }
        // Friction: cloth that touched the body keeps only a little of how it was sliding (its
        // previous place moved most of the way to where it is now).
        for i in 0..self.positions.len() {
            if std::mem::take(&mut self.touching[i]) {
                let (p, q) = (self.positions[i], self.previous[i]);
                let f = self.friction;
                self.previous[i] = [q[0] + (p[0] - q[0]) * f, q[1] + (p[1] - q[1]) * f, q[2] + (p[2] - q[2]) * f];
            }
        }
    }

    /// How much the most stretched link is longer than it was cut, as a share (0.02: 2%).
    /// Links shorter than half the typical one (two particles at the edge, moved onto the outline
    /// close together) are left out: a millimetre on them reads as a large share and says little.
    pub fn worst_stretch(&self) -> f64 {
        let typical = self.typical_link();
        self.links
            .iter()
            .filter(|l| l.2 >= typical * 0.5)
            .map(|&(a, b, rest, _)| {
                let (pa, pb) = (self.positions[a as usize], self.positions[b as usize]);
                let len = ((pb[0] - pa[0]).powi(2) + (pb[1] - pa[1]).powi(2) + (pb[2] - pa[2]).powi(2)).sqrt();
                (len - rest) / rest
            })
            .fold(0.0, f64::max)
    }

    /// The middle length of the links (the grid's step, for a panel's stretch links).
    fn typical_link(&self) -> f64 {
        let mut r: Vec<f64> = self.links.iter().map(|l| l.2).filter(|r| *r > 0.0).collect();
        if r.is_empty() {
            return 0.0;
        }
        r.sort_by(f64::total_cmp);
        r[r.len() / 2]
    }

    /// How much larger each triangle is than it was cut (its area over its area on the pattern,
    /// less 1: 0.05 is 5% larger), for the drape view's tightness colours.
    pub fn tightness(&self) -> Vec<f64> {
        let area3 = |a: [f64; 3], b: [f64; 3], c: [f64; 3]| {
            let (u, v) = ([b[0] - a[0], b[1] - a[1], b[2] - a[2]], [c[0] - a[0], c[1] - a[1], c[2] - a[2]]);
            let n = [u[1] * v[2] - u[2] * v[1], u[2] * v[0] - u[0] * v[2], u[0] * v[1] - u[1] * v[0]];
            (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt() / 2.0
        };
        let cut: Vec<f64> = self
            .triangles
            .iter()
            .map(|t| {
                let [a, b, c] = t.map(|i| self.pattern[i as usize].0);
                ((b - a).cross(c - a) / 2.0).abs()
            })
            .collect();
        let mut out: Vec<f64> = self
            .triangles
            .iter()
            .zip(&cut)
            .map(|(t, cut)| {
                let [p, q, r] = t.map(|i| self.positions[i as usize]);
                if *cut > 1e-12 { area3(p, q, r) / cut - 1.0 } else { 0.0 }
            })
            .collect();
        // A sliver (a quarter of the middle triangle's size or less, at the edge where particles
        // were moved onto the outline) takes the mean of the whole triangles round its corners:
        // its own share swings wildly for a millimetre's movement.
        let mut sorted = cut.clone();
        sorted.sort_by(f64::total_cmp);
        let middle = sorted.get(sorted.len() / 2).copied().unwrap_or(0.0);
        let sliver: Vec<bool> = cut.iter().map(|c| *c < middle * 0.25).collect();
        let mut at = vec![(0.0, 0usize); self.positions.len()];
        for (k, t) in self.triangles.iter().enumerate() {
            if !sliver[k] {
                for &i in t {
                    at[i as usize].0 += out[k];
                    at[i as usize].1 += 1;
                }
            }
        }
        for (k, t) in self.triangles.iter().enumerate() {
            if sliver[k] {
                let (sum, n) = t.iter().fold((0.0, 0), |(s, n), &i| (s + at[i as usize].0, n + at[i as usize].1));
                out[k] = if n > 0 { sum / n as f64 } else { 0.0 };
            }
        }
        out
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
            out_of(p, (0.0, 0.0), self.0, self.0, 0.0)
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
    fn the_edge_of_a_panel_is_on_its_outline() {
        // A trapezoid's slanted sides: every particle at the grid's edge is on the outline (or a
        // quarter step from another that is), none outside it.
        let t = vec![Pt::new(100.0, 0.0), Pt::new(300.0, 0.0), Pt::new(400.0, 300.0), Pt::new(0.0, 300.0)];
        let p = panel(&t, 15.0).unwrap();
        let on = |q: Pt| (nearest_on(&t, q) - q).len() < 1e-9;
        let on_count = p.points.iter().filter(|q| on(**q)).count();
        assert!(on_count >= 2 * (300 / 15) + 2 * (200 / 15), "{on_count}");
        assert!(p.points.iter().all(|q| on(*q) || inside(&t, *q)));
        // The particle nearest a point of the slanted side is on it.
        let side = Pt::new(50.0, 150.0);
        assert!(on(p.points[p.nearest(side).unwrap() as usize]));
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
        let waist = *a.ring("waist").unwrap();
        let (wa, _) = waist.semi_axes();
        // Points inside the waist and a knee come out onto the surface, 2 mm clear, further from
        // the middle than they were; one well clear stays.
        // Out, and within about a millimetre of the surface (a Newton step may go a hair past).
        let on = |rings: &[Ring], q: [f64; 3]| inside_by(rings, q, 2.0).is_some_and(|f| (-1e-9..0.02).contains(&f));
        for p in [[0.0, 0.0, waist.z], [wa * 0.5, 0.0, waist.z], [0.0, -30.0, waist.z - 50.0]] {
            let q = a.push_out(p);
            assert!(on(&a.torso, q), "{p:?} {q:?}");
            assert!(q[0].hypot(q[1]) > p[0].hypot(p[1]));
        }
        assert_eq!(a.push_out([wa + 50.0, 0.0, waist.z]), [wa + 50.0, 0.0, waist.z]);
        let knee = a.legs[0][1];
        let k = a.push_out([knee.x, 0.0, knee.z]);
        assert!(on(&a.legs[0], k), "{k:?}");
        // Where the body widens downward (the waist to the hips), cloth is pushed up as well as
        // out: the surface holds it.
        let hip = *a.ring("hip").unwrap();
        let mid = (waist.z + hip.z) / 2.0;
        let q = a.push_out([0.0, 50.0, mid]);
        assert!(q[2] > mid, "{q:?}");
    }
}
