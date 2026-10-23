//! Design tools (PF-5) on a piece's outline: pivot a dart to somewhere else on the outline, and
//! split an outline into two pieces (a princess seam through the bust point). The outline is a
//! closed ring of [`Knot`]s, as the host's path anchors: the host converts.
//!
//! **Rotating a dart** (the pivot method of flat pattern cutting: the part of the pattern between
//! the dart and the new dart's position swings round the dart point until the old dart closes,
//! and the new one opens by the same angle). The dart is its point (the apex) and the two anchors
//! either side of it (its legs). The new dart opens at an anchor on the outline (the host inserts
//! one where the user points). Either side of the outline can swing: by default the side with
//! fewer anchors, so most of the piece (the centre front with its grain) stays put. With an
//! `amount` below 1 only that share of the dart moves and the rest stays open (two darts).
//!
//! **Slash and spread** cuts the outline straight between two anchors and opens it: hinged at
//! the far end (flare: the gap closes to nothing there) or parallel (even fullness). A knife
//! pleat is a parallel spread of twice its depth, with its fold and placement lines; an inverted
//! pleat four times, both edges folded to its middle.
//!
//! **Facings** copy a run of the outline (a neckline, an armhole) and move it inward by the
//! facing's width for the facing's inner edge; its ends follow the seams beside the run.
//!
//! **Gathering marks**: a wavy line just inside an edge between two ticks, where it is gathered.
//!
//! **Flat collars** (Peter Pan), from the steps published at
//! <https://www.thecreativecurator.com/how-to-draft-a-peter-pan-collar-pattern/>: the front
//! neckline laid against the back's at the neck point, the shoulders overlapping by 2 to 4 cm, the
//! neckline lowered 1 cm when the bodice is a block, and the collar drawn its depth out from the
//! joined neckline (6 cm in that example), its front rounded (a shape the page leaves to the
//! drafter, chosen here).
//!
//! **Patch pockets**: the pocket piece with its hem folded inside at the top, and the finished
//! shape to mark on the garment.
//!
//! **Splitting** cuts the ring between two anchors into two rings, each closed by a straight seam
//! between them; when the two anchors are at the same place (a bust point two darts share) the
//! seam is the darts' legs on each side and nothing is added.
use crate::Pt;

/// An anchor of the outline: its point and its two handles (absolute; a handle equal to the point
/// is none).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Knot {
    pub p: Pt,
    pub h_in: Pt,
    pub h_out: Pt,
}

impl Knot {
    pub const fn corner(p: Pt) -> Self {
        Knot { p, h_in: p, h_out: p }
    }
    fn rotated(self, about: Pt, angle: f64) -> Self {
        let r = |q: Pt| rotate(q, about, angle);
        Knot { p: r(self.p), h_in: r(self.h_in), h_out: r(self.h_out) }
    }
}

/// Which side of the outline swings round the dart point.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Side {
    /// The side with fewer anchors (the forward side on a tie).
    Auto,
    /// From the dart's second leg forward to the new dart.
    After,
    /// From the new dart forward to the dart's first leg.
    Before,
}

/// Where on the outline a design step acts: an anchor, or a point on an edge where an anchor is
/// put first (edge `i` runs from anchor `i` to the next; `t` along its curve).
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum StepPoint {
    Anchor(usize),
    OnEdge { edge: usize, t: f64 },
}

/// A design step made on a drafted block part (PF-5.20), kept with it so it is made again when the
/// block is drawn again (the measurements or the block's options changed). Anchors are the
/// outline's as it was before the step; lengths are in mm.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "step", rename_all = "camelCase")]
pub enum DesignStep {
    /// [`rotate_dart`]: the dart at `apex` moved to `to` (`amount` of it, `side` swinging).
    #[serde(rename_all = "camelCase")]
    RotateDart { apex: usize, to: StepPoint, amount: f64, side: Side },
    /// [`shorten_dart`]: the dart's point `by` mm toward its opening.
    #[serde(rename_all = "camelCase")]
    ShortenDart { apex: usize, by: f64 },
    /// [`curve_dart`]: the dart's legs bowed `depth` mm (more than 0 away from its centre line).
    #[serde(rename_all = "camelCase")]
    CurveDart { apex: usize, depth: f64 },
    /// [`spread`]: slashed from `from` to `to` (`to` placed after `from`) and opened `amount` mm.
    #[serde(rename_all = "camelCase")]
    Spread { from: StepPoint, to: StepPoint, amount: f64, mode: Spread },
    /// The seam from anchor `from` to `to` made `length` mm long by moving its end along it.
    #[serde(rename_all = "camelCase")]
    MatchLength { from: usize, to: usize, length: f64 },
    /// [`split`] between anchors `a` and `b`, keeping piece `keep` (0 or 1) of the two.
    #[serde(rename_all = "camelCase")]
    Split { a: usize, b: usize, keep: usize },
}

/// Why a design tool can't do what was asked.
#[derive(Clone, Debug, PartialEq)]
pub enum DesignError {
    /// An anchor index past the outline's end, or anchors that don't make the operation.
    BadAnchor(&'static str),
    /// The amount is not a number in (0, 1].
    BadAmount,
}

impl std::fmt::Display for DesignError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DesignError::BadAnchor(why) => f.write_str(why),
            DesignError::BadAmount => f.write_str("the amount is a share of the dart, more than 0 and at most 1"),
        }
    }
}

impl std::error::Error for DesignError {}

fn rotate(q: Pt, about: Pt, angle: f64) -> Pt {
    let (s, c) = angle.sin_cos();
    let d = q - about;
    Pt::new(about.x + d.x * c - d.y * s, about.y + d.x * s + d.y * c)
}

fn angle_of(v: Pt) -> f64 {
    v.y.atan2(v.x)
}

/// The angle that turns `from` onto `to` about `about`, in (-π, π].
fn turn(about: Pt, from: Pt, to: Pt) -> f64 {
    let mut a = angle_of(to - about) - angle_of(from - about);
    let tau = std::f64::consts::TAU;
    while a > std::f64::consts::PI {
        a -= tau;
    }
    while a <= -std::f64::consts::PI {
        a += tau;
    }
    a
}

/// The anchors from `from` forward to `to`, both included.
fn run(n: usize, from: usize, to: usize) -> impl Iterator<Item = usize> {
    let len = (to + n - from) % n + 1;
    (0..len).map(move |k| (from + k) % n)
}

fn out_corner(mut k: Knot) -> Knot {
    k.h_out = k.p;
    k
}

fn in_corner(mut k: Knot) -> Knot {
    k.h_in = k.p;
    k
}

/// Pivot the dart whose point is anchor `apex` (its legs the anchors either side) so it opens at
/// anchor `at` instead; `amount` of it moves (1: all of it, the old dart closes). The new dart's
/// point is the old one's. Returns the new outline and the new dart point's anchor. When the old
/// dart's legs differ in length, the swung leg's end joins the other leg's end as the dart closes,
/// and the edge beside it takes up the difference (as truing the dart's legs would).
pub fn rotate_dart(ring: &[Knot], apex: usize, at: usize, amount: f64, side: Side) -> Result<(Vec<Knot>, usize), DesignError> {
    let n = ring.len();
    if n < 4 || apex >= n || at >= n {
        return Err(DesignError::BadAnchor("the dart point or the new dart's anchor is not on the outline"));
    }
    if !(amount > 0.0 && amount <= 1.0) {
        return Err(DesignError::BadAmount);
    }
    let (a, b) = ((apex + n - 1) % n, (apex + 1) % n);
    if at == a || at == apex || at == b {
        return Err(DesignError::BadAnchor("the new dart is on the old dart"));
    }
    let pivot = ring[apex].p;
    let theta = turn(pivot, ring[b].p, ring[a].p);
    // Legs in line (no angle, or straight through the point) make no dart.
    if theta.abs() < 1e-9 || std::f64::consts::PI - theta.abs() < 1e-9 {
        return Err(DesignError::BadAnchor("that anchor's neighbours don't make a dart"));
    }
    let after_count = (at + n - b) % n + 1;
    let before_count = (a + n - at) % n + 1;
    let forward = match side {
        Side::After => true,
        Side::Before => false,
        Side::Auto => after_count <= before_count,
    };
    let full = amount >= 1.0;
    let apex_knot = Knot::corner(pivot);
    let mut out = Vec::with_capacity(n + 2);
    let new_apex;
    if forward {
        // b..=at swing so that b lands on a.
        let ang = theta * amount;
        let swung: Vec<Knot> = run(n, b, at).map(|i| ring[i].rotated(pivot, ang)).collect();
        let last = swung.len() - 1;
        if full {
            let mut first = ring[a];
            first.h_out = swung[0].h_out;
            out.push(first);
            out.extend(swung[1..last].iter().copied());
        } else {
            out.push(ring[a]);
            out.push(ring[apex]);
            out.extend(swung[..last].iter().copied());
        }
        out.push(out_corner(swung[last]));
        new_apex = out.len();
        out.push(apex_knot);
        out.push(in_corner(ring[at]));
        out.extend(run(n, (at + 1) % n, a).filter(|&i| i != a).map(|i| ring[i]));
    } else {
        // at..=a swing so that a lands on b.
        let ang = -theta * amount;
        let swung: Vec<Knot> = run(n, at, a).map(|i| ring[i].rotated(pivot, ang)).collect();
        let last = swung.len() - 1;
        if full {
            let mut first = ring[b];
            first.h_in = swung[last].h_in;
            out.push(first);
        } else {
            out.push(swung[last]);
            out.push(ring[apex]);
            out.push(ring[b]);
        }
        out.extend(run(n, (b + 1) % n, at).filter(|&i| i != at).map(|i| ring[i]));
        out.push(out_corner(ring[at]));
        new_apex = out.len();
        out.push(apex_knot);
        out.push(in_corner(swung[0]));
        out.extend(swung[1..last].iter().copied());
    }
    Ok((out, new_apex))
}

/// The angle between anchor `i`'s two neighbours as seen from it, 0 to π (small at a dart's
/// point, π on a straight run).
pub fn opening(ring: &[Knot], i: usize) -> Option<f64> {
    let n = ring.len();
    if i >= n || n < 3 {
        return None;
    }
    let p = ring[i].p;
    Some(turn(p, ring[(i + 1) % n].p, ring[(i + n - 1) % n].p).abs())
}

/// A dart shortened by `by` (lengthened when `by` is less than 0): its point, anchor `apex`, moved
/// along the dart's centre line (toward the middle of its opening, between its legs' ends) by `by`,
/// and the legs straight from the same ends, so the seam the dart opens on doesn't change: the
/// way a bust dart is ended short of the bust point (Made by Rae, "Bust Dart Adjustment",
/// https://www.made-by-rae.com/blog/2015/07/bust-dart-adjustment-how-to-shorten-or-lengthen-a-dart:
/// "a new point on the dart's center line one inch away from the original point", the legs
/// redrawn with a straight edge). The legs must be straight; lengths are in the outline's unit.
pub fn shorten_dart(ring: &[Knot], apex: usize, by: f64) -> Result<Vec<Knot>, DesignError> {
    let n = ring.len();
    if apex >= n || n < 4 {
        return Err(DesignError::BadAnchor("the dart's point is past the outline's end"));
    }
    if !by.is_finite() {
        return Err(DesignError::BadAnchor("the length to shorten by is not a number"));
    }
    let (a, b) = (ring[(apex + n - 1) % n], ring[(apex + 1) % n]);
    let k = ring[apex];
    let straight = |from: Knot, from_h: Pt, to: Knot, to_h: Pt| (from_h - from.p).len() < 1e-9 && (to_h - to.p).len() < 1e-9;
    if !straight(a, a.h_out, k, k.h_in) || !straight(k, k.h_out, b, b.h_in) {
        return Err(DesignError::BadAnchor("the dart's legs must be straight lines"));
    }
    if opening(ring, apex).is_none_or(|o| o >= std::f64::consts::FRAC_PI_2) {
        return Err(DesignError::BadAnchor("that point is not a dart's point: its legs open a right angle or wider"));
    }
    let mid = (a.p + b.p) * 0.5;
    let length = (mid - k.p).len();
    let dir = (mid - k.p).unit().ok_or(DesignError::BadAnchor("the dart has no length"))?;
    // At least a millimetre's worth of dart is left (in points or mm, a small length either way).
    if by >= length - 1.0 {
        return Err(DesignError::BadAnchor("the dart is not that long"));
    }
    let mut out = ring.to_vec();
    out[apex] = Knot::corner(k.p + dir * by);
    Ok(out)
}

/// A dart's legs curved, each bowed `depth` at its middle (in the outline's unit): more than 0 bows
/// them away from the dart's centre line (a convex dart, which takes in more fabric between its
/// ends and its point), less than 0 toward it (a concave dart, which gives room there). Threads,
/// "Darts build shape into a garment"
/// (https://www.threadsmagazine.com/project-guides/learn-to-sew/darts-build-shape-into-a-garment):
/// curving the legs inward makes a concave dart that adds room in the dart area, and a convex dart
/// with outward-curving legs removes excess fullness. The legs must be straight; the dart's ends
/// and point stay where they are, so the seams it opens on don't change.
pub fn curve_dart(ring: &[Knot], apex: usize, depth: f64) -> Result<Vec<Knot>, DesignError> {
    let n = ring.len();
    if apex >= n || n < 4 {
        return Err(DesignError::BadAnchor("the dart's point is past the outline's end"));
    }
    if !depth.is_finite() {
        return Err(DesignError::BadAnchor("the depth to curve the legs by is not a number"));
    }
    let (ia, ib) = ((apex + n - 1) % n, (apex + 1) % n);
    let (a, k, b) = (ring[ia], ring[apex], ring[ib]);
    let straight = |from: Knot, from_h: Pt, to: Knot, to_h: Pt| (from_h - from.p).len() < 1e-9 && (to_h - to.p).len() < 1e-9;
    if !straight(a, a.h_out, k, k.h_in) || !straight(k, k.h_out, b, b.h_in) {
        return Err(DesignError::BadAnchor("the dart's legs must be straight lines"));
    }
    if opening(ring, apex).is_none_or(|o| o >= std::f64::consts::FRAC_PI_2) {
        return Err(DesignError::BadAnchor("that point is not a dart's point: its legs open a right angle or wider"));
    }
    // At most a quarter of the dart's opening: a leg's middle is about that far from the centre
    // line, so the legs can't cross.
    if depth.abs() >= (b.p - a.p).len() * 0.25 {
        return Err(DesignError::BadAnchor("the legs can't curve that far: the dart isn't wide enough"));
    }
    let mut out = ring.to_vec();
    // Each leg bowed away from the other leg's end (the dart's inside), by `depth`.
    for (from, to, other) in [(ia, apex, b.p), (apex, ib, a.p)] {
        let d = ring[to].p - ring[from].p;
        let left = Pt::new(d.y, -d.x);
        let inside = left.dot(other - (ring[from].p + d * 0.5)) > 0.0;
        bow(&mut out, from, if inside { -depth } else { depth })?;
    }
    Ok(out)
}

/// Another anchor at the same place as anchor `i` (where two darts share their point).
pub fn coincident(ring: &[Knot], i: usize) -> Option<usize> {
    let p = ring.get(i)?.p;
    (0..ring.len()).find(|&j| j != i && (ring[j].p - p).len() < 1e-9)
}

/// Cut the outline between anchors `i` and `j` into two outlines (the anchors from `i` to `j`,
/// and from `j` back to `i`), each closed by a straight seam between them.
pub fn split(ring: &[Knot], i: usize, j: usize) -> Result<(Vec<Knot>, Vec<Knot>), DesignError> {
    let n = ring.len();
    if i >= n || j >= n || i == j {
        return Err(DesignError::BadAnchor("a split needs two different anchors on the outline"));
    }
    let part = |from: usize, to: usize| -> Result<Vec<Knot>, DesignError> {
        let mut v: Vec<Knot> = run(n, from, to).map(|k| ring[k]).collect();
        let first = v.first().copied();
        let last = v.last().copied();
        if let (Some(f), Some(l)) = (first, last) {
            if (f.p - l.p).len() < 1e-9 {
                // The two ends meet: no seam to add, the ends become one anchor.
                v.pop();
                if let Some(f0) = v.first_mut() {
                    f0.h_in = l.h_in;
                }
            } else {
                if let Some(f0) = v.first_mut() {
                    f0.h_in = f.p;
                }
                if let Some(l0) = v.last_mut() {
                    l0.h_out = l.p;
                }
            }
        }
        if v.len() < 3 {
            return Err(DesignError::BadAnchor("a split leaves a piece with fewer than three anchors"));
        }
        Ok(v)
    };
    Ok((part(i, j)?, part(j, i)?))
}

/// How a slash opens.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Spread {
    /// Hinged at the slash's far end: the gap is `amount` wide at its start and closes to nothing
    /// at the hinge (flare added at a hem, the waist kept).
    Hinge,
    /// Both ends move apart by `amount` (even fullness, room for a pleat or gathers).
    Parallel,
}

/// Slash the outline straight from anchor `from` to anchor `to` and spread it open by `amount`
/// (in the outline's unit). The anchors from `from` forward to `to` stay; the rest move away
/// from them. The gap's edges are straight lines, to true afterwards.
pub fn spread(ring: &[Knot], from: usize, to: usize, amount: f64, how: Spread) -> Result<Vec<Knot>, DesignError> {
    let n = ring.len();
    if n < 4 || from >= n || to >= n || from == to || (to + 1) % n == from || (from + 1) % n == to {
        return Err(DesignError::BadAnchor("a slash runs between two points of the outline that aren't neighbours"));
    }
    let (a, b) = (ring[from].p, ring[to].p);
    let len = (b - a).len();
    if !(amount.is_finite() && amount > 0.0) || len < 1e-9 || (how == Spread::Hinge && amount >= 2.0 * len) {
        return Err(DesignError::BadAmount);
    }
    let mut out: Vec<Knot> = run(n, from, to).map(|i| ring[i]).collect();
    let moved: Vec<Knot> = match how {
        Spread::Hinge => {
            // Turned about the hinge by the angle whose chord at the slash's start is `amount`,
            // away from the anchors that stay.
            let angle = 2.0 * (amount / (2.0 * len)).asin();
            let stay_next = ring[(from + 1) % n].p;
            let away = |sign: f64| (rotate(a, b, sign * angle) - stay_next).len();
            let angle = if away(1.0) >= away(-1.0) { angle } else { -angle };
            // The hinge stays; the edge leaving it turns with the moving side.
            if let Some(h) = out.last_mut() {
                h.h_out = rotate(h.h_out, b, angle);
            }
            run(n, (to + 1) % n, from).map(|i| ring[i].rotated(b, angle)).collect()
        }
        Spread::Parallel => {
            // Moved square to the slash, toward the moving side.
            let moving: Vec<usize> = run(n, to, from).collect();
            let centre = moving.iter().fold(Pt::new(0.0, 0.0), |acc, &i| acc + ring[i].p) * (1.0 / moving.len() as f64);
            let dir = (b - a) * (1.0 / len);
            let mut normal = Pt::new(-dir.y, dir.x);
            if normal.dot(centre - a) < 0.0 {
                normal = normal * -1.0;
            }
            let d = normal * amount;
            if let Some(h) = out.last_mut() {
                h.h_out = h.p;
            }
            let mut m: Vec<Knot> = moving.iter().map(|&i| Knot { p: ring[i].p + d, h_in: ring[i].h_in + d, h_out: ring[i].h_out + d }).collect();
            if let Some(h) = m.first_mut() {
                h.h_in = h.p;
            }
            m
        }
    };
    // Where the gap meets the slash's start, both sides are corners.
    if let Some(first) = out.first_mut() {
        first.h_in = first.p;
    }
    let last = moved.len() - 1;
    out.extend(moved[..last].iter().copied());
    out.push(out_corner(moved[last]));
    Ok(out)
}

/// Twice the outline's signed area (shoelace, on the anchors): its sign says which side is in.
fn signed_area2(ring: &[Knot]) -> f64 {
    let n = ring.len();
    (0..n).map(|i| ring[i].p.cross(ring[(i + 1) % n].p)).sum()
}

/// Where the lines through `a` along `da` and through `b` along `db` cross, if they aren't
/// (nearly) parallel.
fn cross_lines(a: Pt, da: Pt, b: Pt, db: Pt) -> Option<Pt> {
    let den = da.cross(db);
    (den.abs() > 1e-9).then(|| a + da * ((b - a).cross(db) / den))
}

/// The cubic from `p0` to `p3` moved `w` to its left (in the shoelace sense: inward on an outline
/// of positive area), by its control polygon: each leg moved square to itself, the inner
/// controls where the moved legs cross. A handle on its anchor takes the chord's direction.
fn offset_cubic(p0: Pt, p1: Pt, p2: Pt, p3: Pt, w: f64) -> [Pt; 4] {
    let left = |d: Pt| d.unit().map(|u| Pt::new(-u.y, u.x) * w);
    let chord = p3 - p0;
    let Some(nc) = left(chord) else { return [p0, p1, p2, p3] };
    let line = (p1 - p0).len() < 1e-9 && (p3 - p2).len() < 1e-9;
    if line {
        return [p0 + nc, p0 + nc, p3 + nc, p3 + nc];
    }
    let d01 = if (p1 - p0).len() > 1e-9 { p1 - p0 } else { p2 - p0 };
    let d23 = if (p3 - p2).len() > 1e-9 { p3 - p2 } else { p3 - p1 };
    let d12 = if (p2 - p1).len() > 1e-9 { p2 - p1 } else { chord };
    let (n01, n12, n23) = (left(d01).unwrap_or(nc), left(d12).unwrap_or(nc), left(d23).unwrap_or(nc));
    let (q0, q3) = (p0 + n01, p3 + n23);
    let q1 = if (p1 - p0).len() > 1e-9 { cross_lines(q0, d01, p1 + n12, d12).unwrap_or(p1 + n01) } else { q0 };
    let q2 = if (p3 - p2).len() > 1e-9 { cross_lines(p2 + n12, d12, q3, d23).unwrap_or(p2 + n23) } else { q3 };
    [q0, q1, q2, q3]
}

/// A cubic cut into pieces that each turn at most a thirty-second of a turn (a straight one stays
/// whole), so moving each by its control polygon stays close to the true offset.
fn pieces(c: [Pt; 4]) -> Vec<[Pt; 4]> {
    let [p0, p1, p2, p3] = c;
    let start = if (p1 - p0).len() > 1e-9 { p1 - p0 } else { p2 - p0 };
    let end = if (p3 - p2).len() > 1e-9 { p3 - p2 } else { p3 - p1 };
    let turned = if start.len() > 1e-9 && end.len() > 1e-9 { turn(Pt::new(0.0, 0.0), start, end).abs() } else { 0.0 };
    let count = ((turned / (std::f64::consts::PI / 16.0)).ceil() as usize).clamp(1, 32);
    let mut out = Vec::with_capacity(count);
    let mut rest = c;
    for k in 0..count {
        if k + 1 == count {
            out.push(rest);
            break;
        }
        let t = 1.0 / (count - k) as f64;
        let [a, b, cc, d] = rest;
        let lerp = |x: Pt, y: Pt| x + (y - x) * t;
        let (ab, bc, cd) = (lerp(a, b), lerp(b, cc), lerp(cc, d));
        let (abc, bcd) = (lerp(ab, bc), lerp(bc, cd));
        let m = lerp(abc, bcd);
        out.push([a, ab, abc, m]);
        rest = [m, bcd, cd, d];
    }
    out
}

/// The open run of knots `run` moved `w` to its left (in the shoelace sense), segment by
/// segment. Where neighbours don't meet (a corner), the outside of the corner is joined by a
/// circular arc about it, the inside trimmed to where the two cross.
fn offset_run(run: &[Knot], w: f64) -> Vec<Knot> {
    let mut out: Vec<Knot> = Vec::new();
    for pair in run.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        for [p0, p1, p2, p3] in pieces([a.p, a.h_out, b.h_in, b.p]) {
            let [q0, q1, q2, q3] = offset_cubic(p0, p1, p2, p3, w);
            let t2 = (q1 - q0).unit().or_else(|| (q3 - q0).unit());
            match out.last_mut() {
                None => out.push(Knot { p: q0, h_in: q0, h_out: q1 }),
                Some(last) if (last.p - q0).len() < 1e-6 => last.h_out = q1 + (last.p - q0),
                Some(last) => {
                    let t1 = (last.p - last.h_in).unit();
                    match (t1, t2) {
                        // Outside the corner (it turns away from the offset's side): an arc.
                        (Some(t1), Some(t2)) if t1.cross(t2) * w < 0.0 => {
                            let sweep = turn(p0, last.p, q0).abs();
                            let k = 4.0 / 3.0 * (sweep / 4.0).tan() * w.abs();
                            last.h_out = last.p + t1 * k;
                            out.push(Knot { p: q0, h_in: q0 - t2 * k, h_out: q1 });
                        }
                        // Inside: both trimmed to where they cross, when that is near.
                        (Some(t1), Some(t2)) => match cross_lines(last.p, t1, q0, t2).filter(|m| (*m - p0).len() < 2.0 * w.abs()) {
                            Some(m) => {
                                last.p = m;
                                last.h_out = q1 + (m - q0);
                            }
                            None => out.push(Knot { p: q0, h_in: q0, h_out: q1 }),
                        },
                        _ => out.push(Knot { p: q0, h_in: q0, h_out: q1 }),
                    }
                }
            }
            out.push(Knot { p: q3, h_in: q2, h_out: q3 });
        }
    }
    out
}

/// A facing for the outline's edge from anchor `from` forward to anchor `to`: a piece whose
/// outer edge is that run of the outline (curves and all) and whose inner edge is the run moved
/// `width` inward. Its ends follow the outline's edges either side (a neck facing ends along the
/// shoulder seam and the centre front), or run square to the edge where those turn away.
pub fn facing(ring: &[Knot], from: usize, to: usize, width: f64) -> Result<Vec<Knot>, DesignError> {
    let n = ring.len();
    if n < 3 || from >= n || to >= n || from == to {
        return Err(DesignError::BadAnchor("a facing runs between two different points of the outline"));
    }
    if !(width.is_finite() && width > 0.0) {
        return Err(DesignError::BadAmount);
    }
    let area = signed_area2(ring);
    if area.abs() < 1e-9 {
        return Err(DesignError::BadAnchor("the outline encloses no area"));
    }
    let w = width * area.signum();
    let idx: Vec<usize> = run(n, from, to).collect();
    if idx.len() == n + 1 || idx.len() > n {
        return Err(DesignError::BadAnchor("a facing runs along part of the outline"));
    }
    let mut inner = offset_run(&idx.iter().map(|&i| ring[i]).collect::<Vec<_>>(), w);
    // The ends along the neighbouring edges: where the inner edge's end tangent meets the line of
    // the outline's next edge out, if that is within a few widths; else square.
    let (first, last) = (inner[0], inner[inner.len() - 1]);
    let start_dir = if (first.h_out - first.p).len() > 1e-9 { first.h_out - first.p } else { inner[1].p - first.p };
    let end_dir = if (last.p - last.h_in).len() > 1e-9 { last.p - last.h_in } else { last.p - inner[inner.len() - 2].p };
    let before = ring[(from + n - 1) % n].p - ring[from].p;
    let after = ring[(to + 1) % n].p - ring[to].p;
    let limit = 3.0 * width;
    let start =
        cross_lines(ring[from].p, before, first.p, start_dir).filter(|q| (*q - ring[from].p).len() < limit && (*q - ring[from].p).dot(before) > 0.0);
    let end = cross_lines(ring[to].p, after, last.p, end_dir).filter(|q| (*q - ring[to].p).len() < limit && (*q - ring[to].p).dot(after) > 0.0);
    if let Some(q) = start {
        let k = &mut inner[0];
        let d = q - k.p;
        *k = Knot { p: q, h_in: q, h_out: k.h_out + d };
    }
    if let Some(q) = end {
        let i = inner.len() - 1;
        let k = &mut inner[i];
        let d = q - k.p;
        *k = Knot { p: q, h_in: k.h_in + d, h_out: q };
    }
    // Round the ring: the outline's run, then the inner edge back (handles swapped).
    let mut out: Vec<Knot> = idx.iter().map(|&i| ring[i]).collect();
    if let Some(f) = out.first_mut() {
        f.h_in = f.p;
    }
    if let Some(l) = out.last_mut() {
        l.h_out = l.p;
    }
    out.extend(inner.iter().rev().map(|k| Knot { p: k.p, h_in: k.h_out, h_out: k.h_in }));
    if let Some(l) = out.last_mut() {
        l.h_out = l.p;
    }
    let m = idx.len();
    if let Some(k) = out.get_mut(m) {
        k.h_in = k.p;
    }
    Ok(out)
}

/// Which pleat.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PleatKind {
    /// One fold, brought over toward the side that stays: twice the depth of fabric.
    Knife,
    /// Two folds brought in to meet at the middle on the right side (the fabric behind): four
    /// times the depth.
    Inverted,
    /// An inverted pleat turned over: two folds a depth in from the spread's edges, each brought
    /// out a depth past its edge, so they show as a box twice the depth wide (the fabric meets
    /// behind it): four times the depth.
    Box,
}

/// A pleat's marks: where to fold, where the folds go, and an arrow from each fold to its
/// placement (each a polyline).
#[derive(Clone, Debug, PartialEq)]
pub struct PleatMarks {
    /// The fold lines (drawn solid).
    pub folds: Vec<[Pt; 2]>,
    /// The placement lines the folds are brought to (drawn dashed).
    pub placements: Vec<[Pt; 2]>,
    /// One arrow per fold, across the middle from the fold to its placement: shaft, then head.
    pub arrows: Vec<[Vec<Pt>; 2]>,
}

/// An arrow from the middle of line `from` toward the middle of line `to`, stopping short of
/// both; `head` is the head's length.
fn arrow(from: [Pt; 2], to: [Pt; 2], head: f64) -> [Vec<Pt>; 2] {
    let (fm, pm) = ((from[0] + from[1]) * 0.5, (to[0] + to[1]) * 0.5);
    let across = pm - fm;
    let (start, tip) = (fm + across * 0.1, fm + across * 0.9);
    let head = head.min(across.len() * 0.4);
    let back = (start - tip).unit().unwrap_or(Pt::new(-1.0, 0.0)) * head;
    let side = Pt::new(-back.y, back.x) * 0.5;
    [vec![start, tip], vec![tip + back + side, tip, tip + back - side]]
}

/// A pleat `depth` deep along the slash from anchor `from` to anchor `to`: the outline spread
/// parallel by twice the depth for a knife pleat (folding toward the side that stays, from
/// `from` forward to `to`), or four times for an inverted pleat (both edges of the spread
/// folded to its middle) or a box pleat (folds a depth in from both edges, brought out a depth
/// past them). `head` is the arrows' head length.
pub fn pleat(ring: &[Knot], from: usize, to: usize, depth: f64, kind: PleatKind, head: f64) -> Result<(Vec<Knot>, PleatMarks), DesignError> {
    let width = match kind {
        PleatKind::Knife => 2.0,
        PleatKind::Inverted | PleatKind::Box => 4.0,
    } * depth;
    let out = spread(ring, from, to, width, Spread::Parallel)?;
    let n = ring.len();
    let stay = (to + n - from) % n + 1;
    // The spread's two edges: where the slash was, and where the side that moved now starts.
    let moved = [out[out.len() - 1].p, out[stay].p];
    let slash = [ring[from].p, ring[to].p];
    let marks = match kind {
        PleatKind::Knife => PleatMarks { folds: vec![moved], placements: vec![slash], arrows: vec![arrow(moved, slash, head)] },
        PleatKind::Inverted => {
            let middle = [(moved[0] + slash[0]) * 0.5, (moved[1] + slash[1]) * 0.5];
            PleatMarks { folds: vec![slash, moved], placements: vec![middle], arrows: vec![arrow(slash, middle, head), arrow(moved, middle, head)] }
        }
        PleatKind::Box => {
            // Across the spread, toward the side that moved.
            let across = (moved[0] - slash[0]).unit().unwrap_or(Pt::new(1.0, 0.0)) * depth;
            let shift = |l: [Pt; 2], by: f64| [l[0] + across * by, l[1] + across * by];
            let folds = [shift(slash, 1.0), shift(moved, -1.0)];
            let placements = [shift(slash, -1.0), shift(moved, 1.0)];
            PleatMarks {
                folds: folds.to_vec(),
                placements: placements.to_vec(),
                arrows: vec![arrow(folds[0], placements[0], head), arrow(folds[1], placements[1], head)],
            }
        }
    };
    Ok((out, marks))
}

/// The point at parameter `t` of the cubic `a`, `c1`, `c2`, `b`, and its direction there.
fn cubic_at(a: Pt, c1: Pt, c2: Pt, b: Pt, t: f64) -> (Pt, Pt) {
    let m = 1.0 - t;
    let p = a * (m * m * m) + c1 * (3.0 * m * m * t) + c2 * (3.0 * m * t * t) + b * (t * t * t);
    let d = (c1 - a) * (3.0 * m * m) + (c2 - c1) * (6.0 * m * t) + (b - c2) * (3.0 * t * t);
    let d = if d.len() > 1e-12 { d } else { b - a };
    (p, d)
}

/// The outline's run from anchor `from` forward to anchor `to` as points about `step` apart,
/// each with its unit direction.
fn walk(ring: &[Knot], from: usize, to: usize, step: f64) -> Vec<(Pt, Pt)> {
    let n = ring.len();
    let idx: Vec<usize> = run(n, from, to).collect();
    let mut out = Vec::new();
    for pair in idx.windows(2) {
        let (a, b) = (ring[pair[0]], ring[pair[1]]);
        let chord = (b.p - a.p).len() + (a.h_out - a.p).len() + (b.h_in - b.p).len();
        let count = ((chord / step).ceil() as usize).clamp(1, 10_000);
        let first = usize::from(!out.is_empty());
        for k in first..=count {
            let (p, d) = cubic_at(a.p, a.h_out, b.h_in, b.p, k as f64 / count as f64);
            out.push((p, d.unit().unwrap_or(Pt::new(1.0, 0.0))));
        }
    }
    out
}

/// A gathering mark for the outline's edge from anchor `from` forward to anchor `to`: a wavy
/// line `inset` inside the edge (waves `wave` long, a quarter of that high), and a tick across
/// the edge at each end, `inset` long either side of it (where the gathering starts and stops).
/// Returns the polylines: the wave, then the two ticks.
pub fn gather_marks(ring: &[Knot], from: usize, to: usize, inset: f64, wave: f64) -> Result<Vec<Vec<Pt>>, DesignError> {
    let n = ring.len();
    if n < 3 || from >= n || to >= n || from == to {
        return Err(DesignError::BadAnchor("gathering runs between two different points of the outline"));
    }
    if !(inset.is_finite() && inset > 0.0 && wave.is_finite() && wave > 0.0) {
        return Err(DesignError::BadAmount);
    }
    let area = signed_area2(ring);
    if area.abs() < 1e-9 {
        return Err(DesignError::BadAnchor("the outline encloses no area"));
    }
    let side = area.signum();
    let pts = walk(ring, from, to, wave / 8.0);
    let mut along = 0.0;
    let mut wavy = Vec::with_capacity(pts.len());
    for (k, &(p, d)) in pts.iter().enumerate() {
        if k > 0 {
            along += (p - pts[k - 1].0).len();
        }
        let inward = Pt::new(-d.y, d.x) * side;
        let bump = (along / wave * std::f64::consts::TAU).sin() * wave / 4.0;
        wavy.push(p + inward * (inset + bump));
    }
    let tick = |(p, d): (Pt, Pt)| {
        let inward = Pt::new(-d.y, d.x) * side;
        vec![p - inward * inset, p + inward * (2.0 * inset)]
    };
    let (Some(&first), Some(&last)) = (pts.first(), pts.last()) else {
        return Err(DesignError::BadAnchor("the edge has no length"));
    };
    Ok(vec![wavy, tick(first), tick(last)])
}

/// The length of the outline's run from anchor `from` forward to anchor `to`.
pub fn run_length(ring: &[Knot], from: usize, to: usize) -> f64 {
    let n = ring.len();
    if n < 2 || from >= n || to >= n {
        return 0.0;
    }
    run(n, from, to)
        .collect::<Vec<_>>()
        .windows(2)
        .map(|w| {
            let (a, b) = (ring[w[0]], ring[w[1]]);
            pieces([a.p, a.h_out, b.h_in, b.p])
                .iter()
                .map(|[p0, p1, p2, p3]| {
                    let at = |t: f64| cubic_at(*p0, *p1, *p2, *p3, t).0;
                    (0..32).map(|k| (at((k + 1) as f64 / 32.0) - at(k as f64 / 32.0)).len()).sum::<f64>()
                })
                .sum::<f64>()
        })
        .sum()
}

/// Where a flat collar's neckline comes from on one bodice part: its outline, the run of the
/// neckline (from `from` forward to `to`) and the shoulder point (the far end of the shoulder
/// from the neck point).
#[derive(Clone, Copy, Debug)]
pub struct Neckline<'a> {
    pub ring: &'a [Knot],
    pub from: usize,
    pub to: usize,
    pub shoulder: usize,
}

/// A flat collar and where the front landed.
#[derive(Clone, Debug, PartialEq)]
pub struct Collar {
    /// The collar's outline: the neck edge from centre back to centre front, the rounded front,
    /// the outer edge back to centre back, and the centre back line (a fold).
    pub outline: Vec<Knot>,
    /// How long the neck edge is (what is sewn to the neckline).
    pub neck_length: f64,
    /// The front's shoulder point, moved with the front.
    pub front_shoulder: Pt,
}

/// A flat (Peter Pan) collar from a back neckline (centre back forward to the neck point) and
/// a front one (neck point forward to centre front), drafted in the back's place: the front is
/// moved so its neck point is the back's and its shoulder lies along the back's, then turned
/// over the back about the neck point until the shoulder points overlap by `overlap` (measured
/// round the turn). The neckline is lowered by `lower` (into the bodice), and the collar is
/// `width` wide from it, its front rounded to meet the neckline at centre front.
pub fn flat_collar(back: Neckline, front: Neckline, overlap: f64, width: f64, lower: f64) -> Result<Collar, DesignError> {
    for part in [back, front] {
        let n = part.ring.len();
        if n < 3 || part.from >= n || part.to >= n || part.shoulder >= n || part.from == part.to {
            return Err(DesignError::BadAnchor("a neckline runs between two different points of its outline, and the shoulder point is one of them"));
        }
    }
    if !(width.is_finite() && width > 0.0 && overlap.is_finite() && overlap >= 0.0 && lower.is_finite() && lower >= 0.0) {
        return Err(DesignError::BadAmount);
    }
    let area = signed_area2(back.ring);
    if area.abs() < 1e-9 {
        return Err(DesignError::BadAnchor("the back's outline encloses no area"));
    }
    let knots = |part: Neckline| run(part.ring.len(), part.from, part.to).map(|i| part.ring[i]).collect::<Vec<Knot>>();
    let (b, f) = (knots(back), knots(front));
    let (pb, pf) = (back.ring[back.to].p, front.ring[front.from].p);
    let shoulder_b = back.ring[back.shoulder].p - pb;
    let shoulder_f = front.ring[front.shoulder].p - pf;
    let (Some(db), Some(df)) = (shoulder_b.unit(), shoulder_f.unit()) else {
        return Err(DesignError::BadAnchor("a shoulder point is on its neck point"));
    };
    // Turn the front's shoulder onto the back's; reflect it if its body would then lie on the
    // back's side of the shoulder (the halves were drafted facing each other).
    let a0 = turn(Pt::new(0.0, 0.0), df, db);
    let back_side = db.cross(b[0].p - pb).signum();
    let cf = rotate(f[f.len() - 1].p - pf, Pt::new(0.0, 0.0), a0);
    let reflect = db.cross(cf).signum() == back_side;
    // Then over the back by the overlap.
    let extra = back_side * overlap / shoulder_f.len();
    let place = |q: Pt| {
        let v = rotate(q - pf, Pt::new(0.0, 0.0), a0);
        let v = if reflect { db * (2.0 * v.dot(db)) - v } else { v };
        pb + rotate(v, Pt::new(0.0, 0.0), extra)
    };
    let moved = |k: Knot| Knot { p: place(k.p), h_in: place(k.h_in), h_out: place(k.h_out) };
    // The neckline: the back's run, the neck point joined, the front's run moved.
    let mut neck: Vec<Knot> = b[..b.len() - 1].to_vec();
    let fm: Vec<Knot> = f.iter().map(|&k| moved(k)).collect();
    neck.push(Knot { p: pb, h_in: b[b.len() - 1].h_in, h_out: fm[0].h_out });
    neck.extend(fm[1..].iter().copied());
    if let Some(k) = neck.first_mut() {
        k.h_in = k.p;
    }
    if let Some(k) = neck.last_mut() {
        k.h_out = k.p;
    }
    // Into the bodice is to the neckline's left times the back outline's orientation.
    let body = area.signum();
    // Both edges from the neckline as drawn (an edge offset twice would carry the first
    // offset's corner joins out as spikes).
    let outer = offset_run(&neck, (lower + width) * body);
    if lower > 0.0 {
        neck = offset_run(&neck, lower * body);
    }
    let neck_length = run_length(&neck, 0, neck.len() - 1);
    let (Some(&n_end), Some(&o_end), Some(&o_start)) = (neck.last(), outer.last(), outer.first()) else {
        return Err(DesignError::BadAnchor("the neckline has no length"));
    };
    // The rounded front (a shape the source leaves to the drafter): out from centre front square
    // to the neckline, into the outer edge along it.
    let out_dir = (o_end.p - n_end.p).unit().unwrap_or(Pt::new(0.0, 1.0));
    let along = if (o_end.p - o_end.h_in).len() > 1e-9 { o_end.p - o_end.h_in } else { o_end.p - outer[outer.len() - 2].p };
    let along = along.unit().unwrap_or(Pt::new(1.0, 0.0));
    let mut outline = neck.clone();
    if let Some(k) = outline.last_mut() {
        k.h_out = n_end.p + out_dir * (width * 0.55);
    }
    let mut back_round: Vec<Knot> = outer.iter().rev().map(|k| Knot { p: k.p, h_in: k.h_out, h_out: k.h_in }).collect();
    if let Some(k) = back_round.first_mut() {
        k.h_in = o_end.p + along * (width * 0.3);
    }
    if let Some(k) = back_round.last_mut() {
        k.h_out = o_start.p;
    }
    outline.extend(back_round);
    Ok(Collar { outline, neck_length, front_shoulder: place(front.ring[front.shoulder].p) })
}

/// A patch pocket's bottom corners.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PocketCorners {
    Square,
    /// Rounded with this radius.
    Round(f64),
    /// A point at the bottom middle, this far below the sides' ends.
    Point(f64),
}

/// A patch pocket: the piece (its hem included) and the shape it has finished.
#[derive(Clone, Debug, PartialEq)]
pub struct Pocket {
    /// The piece: `width` wide and the depth and the hem tall, from its top-left corner at the
    /// origin (y down), clockwise on screen. Its seam allowance comes from Make Pattern Piece.
    pub piece: Vec<Knot>,
    /// Where the hem folds inside, across the piece.
    pub fold: [Pt; 2],
    /// The finished pocket (the piece below the fold), top-left at the origin: what is marked
    /// on the garment.
    pub finished: Vec<Knot>,
}

/// A patch pocket `width` wide and `depth` deep when finished, with a `hem` folded inside at
/// the top.
pub fn patch_pocket(width: f64, depth: f64, hem: f64, corners: PocketCorners) -> Result<Pocket, DesignError> {
    let ok = |v: f64| v.is_finite() && v > 0.0;
    if !(ok(width) && ok(depth) && hem.is_finite() && hem >= 0.0) {
        return Err(DesignError::BadAmount);
    }
    match corners {
        PocketCorners::Round(r) if !(ok(r) && 2.0 * r <= width && r <= depth) => return Err(DesignError::BadAmount),
        PocketCorners::Point(p) if !(ok(p) && p < depth) => return Err(DesignError::BadAmount),
        _ => {}
    }
    // The outline from `top` down to `bottom`: across the top, down the right side, across the
    // bottom, up the left side.
    let shape = |top: f64, bottom: f64| -> Vec<Knot> {
        let mut v = vec![Knot::corner(Pt::new(0.0, top)), Knot::corner(Pt::new(width, top))];
        match corners {
            PocketCorners::Square => {
                v.push(Knot::corner(Pt::new(width, bottom)));
                v.push(Knot::corner(Pt::new(0.0, bottom)));
            }
            PocketCorners::Round(r) => {
                // A quarter circle as a cubic: handles 0.5523 of the radius.
                let k = r * 0.552_284_749_8;
                v.push(Knot { p: Pt::new(width, bottom - r), h_in: Pt::new(width, bottom - r), h_out: Pt::new(width, bottom - r + k) });
                v.push(Knot { p: Pt::new(width - r, bottom), h_in: Pt::new(width - r + k, bottom), h_out: Pt::new(width - r, bottom) });
                v.push(Knot { p: Pt::new(r, bottom), h_in: Pt::new(r, bottom), h_out: Pt::new(r - k, bottom) });
                v.push(Knot { p: Pt::new(0.0, bottom - r), h_in: Pt::new(0.0, bottom - r + k), h_out: Pt::new(0.0, bottom - r) });
            }
            PocketCorners::Point(p) => {
                v.push(Knot::corner(Pt::new(width, bottom - p)));
                v.push(Knot::corner(Pt::new(width / 2.0, bottom)));
                v.push(Knot::corner(Pt::new(0.0, bottom - p)));
            }
        }
        v
    };
    Ok(Pocket { piece: shape(0.0, hem + depth), fold: [Pt::new(0.0, hem), Pt::new(width, hem)], finished: shape(0.0, depth) })
}

/// Where a curve meets a seam on one piece: its outline, the anchor the curve and the seam
/// share, and the anchor at the seam's other end (one of that anchor's neighbours).
#[derive(Clone, Copy, Debug)]
pub struct Junction<'a> {
    pub ring: &'a [Knot],
    pub at: usize,
    pub seam: usize,
}

impl Junction<'_> {
    /// Whether the curve leaves `at` forward (the seam is behind it), and the curve's direction
    /// there: its handle, or toward the curve's next anchor when it has none.
    fn curve(&self) -> Result<(bool, Pt), DesignError> {
        let n = self.ring.len();
        if n < 3 || self.at >= n || self.seam >= n {
            return Err(DesignError::BadAnchor("the meeting point and the seam's other end are points of the outline"));
        }
        let k = self.ring[self.at];
        let forward = if (self.at + n - 1) % n == self.seam {
            true
        } else if (self.at + 1) % n == self.seam {
            false
        } else {
            return Err(DesignError::BadAnchor("the seam's other end is a neighbour of the meeting point"));
        };
        let (handle, next) = if forward { (k.h_out, self.ring[(self.at + 1) % n].p) } else { (k.h_in, self.ring[(self.at + n - 1) % n].p) };
        let dir = if (handle - k.p).len() > 1e-9 { handle - k.p } else { next - k.p };
        dir.unit().map(|d| (forward, d)).ok_or(DesignError::BadAnchor("the curve has no length at the meeting point"))
    }
}

/// Smooth the curve across a seam two pieces share: piece `b` is laid against `a` along the
/// seam (meeting points together, `b`'s body on the seam's other side), and both curves' handles
/// at the meeting points are turned to one line, so the curve runs on without a corner once
/// sewn. Handle lengths are kept (a point with no handle gets one a third of the way to the
/// curve's next point). Returns the two new meeting-point knots and the corner there was, in
/// radians.
pub fn smooth_across(a: Junction, b: Junction) -> Result<(Knot, Knot, f64), DesignError> {
    let (fa, ta) = a.curve()?;
    let (fb, tb) = b.curve()?;
    let (pa, pb) = (a.ring[a.at].p, b.ring[b.at].p);
    let (Some(sa), Some(sb)) = ((a.ring[a.seam].p - pa).unit(), (b.ring[b.seam].p - pb).unit()) else {
        return Err(DesignError::BadAnchor("a seam has no length"));
    };
    // b's directions in a's frame: its seam onto a's, its curve on the seam's other side.
    let rot = turn(Pt::new(0.0, 0.0), sb, sa);
    let reflect = sa.cross(rotate(tb, Pt::new(0.0, 0.0), rot)).signum() == sa.cross(ta).signum();
    let to_a = |v: Pt| {
        let v = rotate(v, Pt::new(0.0, 0.0), rot);
        if reflect { sa * (2.0 * v.dot(sa)) - v } else { v }
    };
    let to_b = |v: Pt| {
        let v = if reflect { sa * (2.0 * v.dot(sa)) - v } else { v };
        rotate(v, Pt::new(0.0, 0.0), -rot)
    };
    let tb_a = to_a(tb);
    let corner = std::f64::consts::PI - turn(Pt::new(0.0, 0.0), ta, tb_a).abs();
    let u = (ta - tb_a).unit().ok_or(DesignError::BadAnchor("the curves leave the seam the same way"))?;
    let set = |j: Junction, forward: bool, dir: Pt| -> Knot {
        let n = j.ring.len();
        let mut k = j.ring[j.at];
        let next = if forward { j.ring[(j.at + 1) % n].p } else { j.ring[(j.at + n - 1) % n].p };
        let old = if forward { k.h_out } else { k.h_in };
        let len = if (old - k.p).len() > 1e-9 { (old - k.p).len() } else { (next - k.p).len() / 3.0 };
        if forward {
            k.h_out = k.p + dir * len;
        } else {
            k.h_in = k.p + dir * len;
        }
        k
    };
    Ok((set(a, fa, u), set(b, fb, to_b(u * -1.0)), corner))
}

/// A stand (mandarin) collar: its outline and marks, drafted flat with centre back at (0, 0),
/// the neck edge running along +x and the stand above it (toward -y).
#[derive(Clone, Debug, PartialEq)]
pub struct StandCollar {
    /// The half collar: the neck edge from centre back to the front end, the front end, the top
    /// edge back to centre back, and the centre back line (a fold).
    pub outline: Vec<Knot>,
    /// Where the shoulder seam meets the neck edge (a notch).
    pub shoulder: Pt,
    /// The centre front line across the stand, `extension` back from the front end.
    pub centre_front: [Pt; 2],
    /// How long the top edge is from centre back to centre front (where a shirt collar's fall
    /// is sewn on).
    pub top: f64,
}

/// A stand collar for a neckline `back` long from centre back to the shoulder and `front` from
/// the shoulder to centre front, `depth` high: a band whose neck edge runs straight to the
/// shoulder, then curves up to `rise` above its line at centre front (so it hugs the neck), and
/// runs on straight past centre front by `extension` (half a button stand where the fronts meet
/// edge to edge, all of it where they overlap). The top edge is parallel to the neck edge, the
/// front end square to it. Drafted from the published method in The Creative Curator,
/// <https://www.thecreativecurator.com/draft-mandarin-collar-pattern/>.
pub fn stand_collar(back: f64, front: f64, depth: f64, rise: f64, extension: f64) -> Result<StandCollar, DesignError> {
    let ok = |v: f64| v.is_finite() && v >= 0.0;
    if !(back > 0.0 && front > 0.0 && depth > 0.0 && ok(back) && ok(front) && ok(depth) && ok(rise) && ok(extension)) {
        return Err(DesignError::BadAnchor("the neckline lengths and the stand's height are more than 0, the rise and extension 0 or more"));
    }
    if rise >= front * 0.7 {
        return Err(DesignError::BadAnchor("the rise at centre front is less than the front neckline is long"));
    }
    let s = Pt::new(back, 0.0);
    // The front curve: leaving the shoulder level, like an arc, ending `rise` up; `dx` across
    // so that it is `front` long.
    let curve = |dx: f64| -> [Knot; 2] {
        let c = Pt::new(back + dx, -rise);
        let k = (c - s).len() / 3.0;
        let a = 2.0 * rise.atan2(dx);
        [Knot { p: s, h_in: s, h_out: s + Pt::new(k, 0.0) }, Knot { p: c, h_in: c - Pt::new(a.cos(), -a.sin()) * k, h_out: c }]
    };
    let (mut lo, mut hi) = (0.0, front);
    for _ in 0..60 {
        let mid = (lo + hi) / 2.0;
        if run_length(&curve(mid), 0, 1) < front {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    let [ks, kc] = curve((lo + hi) / 2.0);
    let a = 2.0 * rise.atan2(kc.p.x - back);
    let e = kc.p + Pt::new(a.cos(), -a.sin()) * extension;
    let cb = Knot::corner(Pt::new(0.0, 0.0));
    let mut base = vec![cb, ks, kc];
    if extension > 1e-9 {
        base.push(Knot::corner(e));
    }
    // The top edge, the stand's height above (whichever side of the run that is).
    let mut up = depth;
    let mut top = offset_run(&base, up);
    if top.first().is_some_and(|k| k.p.y > 0.0) {
        up = -depth;
        top = offset_run(&base, up);
    }
    let c_top = kc.p + Pt::new(-a.sin(), -a.cos()) * depth;
    let to_cf = offset_run(&[cb, ks, kc], up);
    let top_length = run_length(&to_cf, 0, to_cf.len().saturating_sub(1));
    let mut outline = base;
    outline.extend(top.iter().rev().map(|k| Knot { p: k.p, h_in: k.h_out, h_out: k.h_in }));
    Ok(StandCollar { outline, shoulder: s, centre_front: [kc.p, c_top], top: top_length })
}

/// An in-seam pocket's bag, drafted on the piece it hangs inside.
#[derive(Clone, Debug, PartialEq)]
pub struct InSeamPocket {
    /// The bag: its seam edge along the garment's seam (from above the opening to below it),
    /// then round its curved edge, inside the piece.
    pub bag: Vec<Knot>,
    /// The opening's ends, as distances along the seam from its start.
    pub opening: [f64; 2],
}

/// The outline the other way round: the same shape, its knots in reverse order.
fn reversed(ring: &[Knot]) -> Vec<Knot> {
    ring.iter().rev().map(|k| Knot { p: k.p, h_in: k.h_out, h_out: k.h_in }).collect()
}

/// An in-seam pocket in the seam from anchor `from` (its top: the waist) to anchor `to`, along
/// the shorter way round the outline: an opening `opening` long starting `top` down the seam,
/// and a bag reaching `width` in from the seam and `depth` below the opening, its seam edge
/// running `allowance` past both ends of the opening (where the bag is stitched on), then square
/// in from the seam by `allowance` before it curves round. After the published methods in
/// Seamwork (<https://www.seamwork.com/articles/design-your-own-in-seam-pockets>) and Doina
/// Alexei (an opening 3 to 5 inches below the waist and 5 to 5½ inches long,
/// <https://www.doinaalexei.com/beginnersewingtutorialblog/video-tutorial-how-to-make-a-sewing-pattern-for-a-skirt-on-seam-pocket>).
#[allow(clippy::too_many_arguments)]
pub fn in_seam_pocket(
    ring: &[Knot],
    from: usize,
    to: usize,
    top: f64,
    opening: f64,
    depth: f64,
    width: f64,
    allowance: f64,
) -> Result<InSeamPocket, DesignError> {
    let n = ring.len();
    if n < 3 || from >= n || to >= n || from == to {
        return Err(DesignError::BadAnchor("the seam runs between two different points of the outline"));
    }
    if ![top, opening, depth, width, allowance].iter().all(|v| v.is_finite() && *v >= 0.0) || opening <= 0.0 || depth <= 0.0 || width <= allowance {
        return Err(DesignError::BadAnchor("the opening, depth and width are more than 0, the width more than the allowance"));
    }
    let back = reversed(ring);
    let (r, f, t) = if run_length(ring, from, to) <= run_length(&back, n - 1 - from, n - 1 - to) {
        (ring.to_vec(), from, to)
    } else {
        (back, n - 1 - from, n - 1 - to)
    };
    let len = run_length(&r, f, t);
    let (a, b) = (top - allowance, top + opening + allowance);
    if a < 0.0 || b > len {
        return Err(DesignError::BadAnchor("the pocket (its opening and the allowance past both ends) fits on the seam"));
    }
    let samples = walk(&r, f, t, (len / 2000.0).max(1e-3));
    let mut dist = Vec::with_capacity(samples.len());
    let mut d = 0.0;
    for (i, (p, _)) in samples.iter().enumerate() {
        if i > 0 {
            d += (*p - samples[i - 1].0).len();
        }
        dist.push(d);
    }
    // The walk's length is the seam's to within its step; scale it to the seam's own.
    let scale = if d > 0.0 { len / d } else { 1.0 };
    let at = |x: f64| -> Pt {
        let x = x / scale;
        let i = dist.partition_point(|&v| v < x).clamp(1, dist.len().saturating_sub(1).max(1));
        let (p0, p1) = (samples[i - 1].0, samples.get(i).map(|s| s.0).unwrap_or(samples[i - 1].0));
        let span = dist.get(i).copied().unwrap_or(dist[i - 1]) - dist[i - 1];
        if span > 1e-12 { p0 + (p1 - p0) * ((x - dist[i - 1]) / span) } else { p0 }
    };
    let (pa, pb, o1, o2) = (at(a), at(b), at(top), at(top + opening));
    // The bag hangs along the seam's way at its lower end (below the hip a side seam runs with
    // the grain), not along the chord of a curved hip.
    let s = (pb - at(b - (b - a) * 0.1)).unit().ok_or(DesignError::BadAnchor("the seam has no length there"))?;
    // Into the piece: left of the seam's way on an outline of positive area.
    let inward = Pt::new(-s.y, s.x) * if signed_area2(&r) >= 0.0 { 1.0 } else { -1.0 };
    let mut bag: Vec<Knot> = (0..=16).map(|k| Knot::corner(at(a + (b - a) * k as f64 / 16.0))).collect();
    let tb = pb + inward * allowance;
    let c = o2 + s * depth + inward * (width * 0.6);
    let dd = o1 + s * (opening * 0.25) + inward * width;
    let ta = pa + inward * allowance;
    let kd = (c - dd).dot(s).abs() * 0.5;
    bag.push(Knot { p: tb, h_in: tb, h_out: tb + s * (depth * 0.55) });
    bag.push(Knot { p: c, h_in: c - inward * (width * 0.3), h_out: c + inward * (width * 0.3) });
    // Up from the widest point no higher than the top tab, so the bag stays below it.
    bag.push(Knot { p: dd, h_in: dd + s * kd, h_out: dd - s * ((dd - ta).dot(s).abs() * 0.9) });
    bag.push(Knot { p: ta, h_in: ta + inward * ((width - allowance) * 0.5), h_out: ta });
    Ok(InSeamPocket { bag, opening: [top, top + opening] })
}

/// How many welts a welt pocket has.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Welts {
    /// One welt filling the opening.
    Single,
    /// Two welts meeting in the middle of the opening (a jetted pocket).
    Double,
}

/// A welt pocket's pieces and marks, laid out with the welt's top-left at (0, 0) and the
/// opening's top-left at (0, 0) for the marks on the garment.
#[derive(Clone, Debug, PartialEq)]
pub struct WeltPocket {
    /// The welt strip, cut on the fold line along its middle (cut one, or two for double welts).
    pub welt: Vec<Knot>,
    /// The welt's fold line.
    pub fold: [Pt; 2],
    /// The two pocket bags: the lower (behind the welt), and the upper, deeper by the opening's
    /// height (it reaches up to the opening's top edge).
    pub bags: [Vec<Knot>; 2],
    /// The opening on the garment: a box the welts fill.
    pub opening: Vec<Knot>,
    /// Where the opening is cut: along its middle, then out to its four corners.
    pub cuts: Vec<[Pt; 2]>,
}

fn rect(x: f64, y: f64, w: f64, h: f64) -> Vec<Knot> {
    [(x, y), (x + w, y), (x + w, y + h), (x, y + h)].iter().map(|&(a, b)| Knot::corner(Pt::new(a, b))).collect()
}

/// A welt pocket with an opening `opening` long and welts `welt` high when finished (one, or two
/// meeting in the middle), its bags `depth` deep below the opening. The welt strip is the
/// opening plus `end` at each end long and twice the welt plus `side` on each long edge high
/// (it is folded in half); the bags are as long as the welt strip. After the published tutorial
/// in The Shapes of Fabric (<https://www.theshapesoffabric.com/2019/06/24/welt-pocket-tutorial/>):
/// 7 mm welts with 7 mm allowances along them and 15 mm at their ends.
pub fn welt_pocket(opening: f64, welt: f64, depth: f64, welts: Welts, side: f64, end: f64) -> Result<WeltPocket, DesignError> {
    if ![opening, welt, depth, side, end].iter().all(|v| v.is_finite() && *v >= 0.0) || opening <= 0.0 || welt <= 0.0 || depth <= 0.0 {
        return Err(DesignError::BadAnchor("the opening, the welt and the depth are more than 0, the allowances 0 or more"));
    }
    let high = match welts {
        Welts::Single => welt,
        Welts::Double => 2.0 * welt,
    };
    if high * 2.0 > opening {
        return Err(DesignError::BadAnchor("the opening is longer than twice its height"));
    }
    let (w, h) = (opening + 2.0 * end, 2.0 * welt + 2.0 * side);
    let gap = 20.0;
    let lower = rect(0.0, h + gap, w, depth + side);
    let upper = rect(w + gap, h + gap, w, depth + high + side);
    // The cut: along the middle, stopping half the height from each end, then out to the corners.
    let (m, i) = (high / 2.0, high / 2.0);
    let (a, b) = (Pt::new(i, m), Pt::new(opening - i, m));
    let cuts = vec![[a, b], [a, Pt::new(0.0, 0.0)], [a, Pt::new(0.0, high)], [b, Pt::new(opening, 0.0)], [b, Pt::new(opening, high)]];
    Ok(WeltPocket {
        welt: rect(0.0, 0.0, w, h),
        fold: [Pt::new(0.0, h / 2.0), Pt::new(w, h / 2.0)],
        bags: [lower, upper],
        opening: rect(0.0, 0.0, opening, high),
        cuts,
    })
}

/// The shape of a pocket flap's lower edge.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FlapShape {
    /// Square corners.
    Square,
    /// Rounded corners, a third of the depth round.
    Rounded,
    /// A point in the middle, a third of the depth below the corners.
    Pointed,
}

impl FlapShape {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "square" => Some(FlapShape::Square),
            "rounded" => Some(FlapShape::Rounded),
            "pointed" => Some(FlapShape::Pointed),
            _ => None,
        }
    }
}

/// A pocket flap (PF-5.19), `width` across and `depth` deep when finished, its top edge the one
/// sewn into the opening: its cutting line (`allowance` out all round) and its stitching line,
/// with the cutting line's top-left at (0, 0), y down. The Shapes of Fabric's welt pocket
/// tutorial leaves the shape to the maker ("Draw a shape you like and then add seam allowance all
/// around") and makes the flap "the same as your finished welt pocket, or maybe 2mm smaller";
/// the corners' rounding and the point's depth (a third of the depth) are chosen here.
pub fn pocket_flap(width: f64, depth: f64, shape: FlapShape, allowance: f64) -> Result<(Vec<Knot>, Vec<Knot>), DesignError> {
    if ![width, depth, allowance].iter().all(|v| v.is_finite()) || width <= 0.0 || depth <= 0.0 || allowance < 0.0 {
        return Err(DesignError::BadAnchor("the flap's width and depth are more than 0, its allowance 0 or more"));
    }
    if depth > width * 2.0 {
        return Err(DesignError::BadAnchor("the flap is deeper than twice its width"));
    }
    let a = allowance;
    // The finished shape, offset by `grow` (the allowance for the cutting line), at (a, a).
    let outline = |grow: f64| -> Vec<Knot> {
        let (x0, y0, x1, y1) = (a - grow, a - grow, a + width + grow, a + depth + grow);
        match shape {
            FlapShape::Square => rect(x0, y0, x1 - x0, y1 - y0),
            FlapShape::Rounded => {
                // The corner's centre stays put: its radius grows with the line.
                let r0 = (depth / 3.0).min(width / 2.0);
                let r = r0 + grow;
                let k = 0.552_284_75 * r;
                let (cx0, cx1, cy) = (a + r0, a + width - r0, a + depth - r0);
                vec![
                    Knot::corner(Pt::new(x0, y0)),
                    Knot::corner(Pt::new(x1, y0)),
                    Knot { p: Pt::new(x1, cy), h_in: Pt::new(x1, cy), h_out: Pt::new(x1, cy + k) },
                    Knot { p: Pt::new(cx1, y1), h_in: Pt::new(cx1 + k, y1), h_out: Pt::new(cx1, y1) },
                    Knot { p: Pt::new(cx0, y1), h_in: Pt::new(cx0, y1), h_out: Pt::new(cx0 - k, y1) },
                    Knot { p: Pt::new(x0, cy), h_in: Pt::new(x0, cy + k), h_out: Pt::new(x0, cy) },
                ]
            }
            FlapShape::Pointed => {
                // The finished pentagon's sides moved out by `grow` (mitred corners).
                let drop = depth / 3.0;
                let fin = [
                    Pt::new(a, a),
                    Pt::new(a + width, a),
                    Pt::new(a + width, a + depth - drop),
                    Pt::new(a + width / 2.0, a + depth),
                    Pt::new(a, a + depth - drop),
                ];
                let n = fin.len();
                let out = |i: usize| {
                    let d = (fin[(i + 1) % n] - fin[i]).unit().unwrap_or(Pt::new(1.0, 0.0));
                    // Clockwise with y down: the outside is to the left of the direction.
                    Pt::new(d.y, -d.x)
                };
                (0..n)
                    .map(|i| {
                        let (n0, n1) = (out((i + n - 1) % n), out(i));
                        let m = n0 + n1;
                        let scale = grow / (1.0 + n0.dot(n1)).max(1e-9);
                        Knot::corner(fin[i] + m * scale)
                    })
                    .collect()
            }
        }
    };
    Ok((outline(a), outline(0.0)))
}

/// A shirt collar's fall (the collar that turns down over its stand), drafted flat with centre
/// back at the neck edge at (0, 0), the neck edge along +x and the fall above it (toward -y).
/// The neck edge is `neck` long (the stand's top edge, centre back to centre front) and curves
/// down by `drop` by its front end; the fall is `height` high at centre back, and its point
/// reaches `out` past the neck edge's end and `point` up from it. After the published method
/// in NIMI's Fashion Design Technology trade theory
/// (<https://nimi.co.in/cits_ebooks1/Fashion_Design_Technology/Fashion_Design_Technology_TT/files/basic-html/page164.html>):
/// a 1½ inch fall, its point 2 inches long and ¾ inch out, the neck edge ½ inch down at the
/// front.
pub fn collar_fall(neck: f64, height: f64, drop: f64, out: f64, point: f64) -> Result<Vec<Knot>, DesignError> {
    if ![neck, height, drop, out, point].iter().all(|v| v.is_finite() && *v >= 0.0) || neck <= 0.0 || height <= 0.0 || point <= 0.0 {
        return Err(DesignError::BadAnchor(
            "the neck edge, the fall and its point are more than 0; the drop and how far the point reaches out 0 or more",
        ));
    }
    if drop >= neck * 0.5 {
        return Err(DesignError::BadAnchor("the drop at the front is less than half the neck edge"));
    }
    let k0 = Pt::new(0.0, 0.0);
    // The neck edge: leaving centre back level, like an arc, ending `drop` down, `neck` long.
    let curve = |dx: f64| -> [Knot; 2] {
        let p = Pt::new(dx, drop);
        let k = (p - k0).len() / 3.0;
        let a = 2.0 * drop.atan2(dx);
        [Knot { p: k0, h_in: k0, h_out: k0 + Pt::new(k, 0.0) }, Knot { p, h_in: p - Pt::new(a.cos(), a.sin()) * k, h_out: p }]
    };
    let (mut lo, mut hi) = (0.0, neck);
    for _ in 0..60 {
        let mid = (lo + hi) / 2.0;
        if run_length(&curve(mid), 0, 1) < neck {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    let [kb, kp] = curve((lo + hi) / 2.0);
    let o = Pt::new(kp.p.x + out, kp.p.y - point);
    let l = Pt::new(0.0, -height);
    Ok(vec![kb, kp, Knot::corner(o), Knot::corner(l)])
}

/// A half outline (a front cut on the fold) opened out into the whole: mirrored across the
/// line through anchors `a` and `b`, which are neighbours on the outline (the centre front edge,
/// either way round). The result starts with the half's own anchors from `b` round to `a` (so
/// anchor `k` of the half is anchor `(k + n - b) % n` of the whole when `a` comes just before
/// `b`), then their mirror images back from `a` to `b`; the edge between them is gone. As a wrap
/// front is drafted: "because the front is asymmetric, don't place it on a center-front fold",
/// the block reflected across centre front (Ploen Patterns,
/// <https://ploenpatterns.com/blogs/news/how-to-draft-a-wrap-blouse-step-by-step-pattern-making-tutorial>;
/// Sewing with Numbers, <https://sewingwithnumbers.substack.com/p/how-to-draft-a-wrap-skirt>).
pub fn opened_out(ring: &[Knot], a: usize, b: usize) -> Result<Vec<Knot>, DesignError> {
    let n = ring.len();
    if n < 3 || a >= n || b >= n {
        return Err(DesignError::BadAnchor("the fold edge's anchors are past the outline's end"));
    }
    // Walk from the far end of the fold edge round to its near end, the edge left out.
    let (from, to) = if (a + 1) % n == b {
        (b, a)
    } else if (b + 1) % n == a {
        (a, b)
    } else {
        return Err(DesignError::BadAnchor("the fold edge's two anchors must be neighbours on the outline"));
    };
    let (p, q) = (ring[from].p, ring[to].p);
    let axis = (q - p).unit().ok_or(DesignError::BadAnchor("the fold edge has no length"))?;
    let mirror = |v: Pt| {
        let d = v - p;
        p + axis * (2.0 * d.dot(axis)) - d
    };
    let half: Vec<Knot> = run(n, from, to).map(|k| ring[k]).collect();
    let mut whole = half.clone();
    // Across the fold the outline carries on as its mirror image: the ends' other handles too.
    if let Some(last) = whole.last_mut() {
        last.h_out = mirror(last.h_in);
    }
    if let Some(first) = whole.first_mut() {
        first.h_in = mirror(first.h_out);
    }
    for k in half[1..half.len() - 1].iter().rev() {
        whole.push(Knot { p: mirror(k.p), h_in: mirror(k.h_out), h_out: mirror(k.h_in) });
    }
    Ok(whole)
}

/// The straight edge from anchor `i` to the next one, bowed by `depth` at its middle toward the
/// left of its direction (a negative depth bows it to the right): a wrap edge drawn "slightly
/// curved and shorter so it stays closer to the body" (Ploen Patterns, as [`opened_out`]).
pub fn bow(ring: &mut [Knot], i: usize, depth: f64) -> Result<(), DesignError> {
    let n = ring.len();
    if i >= n {
        return Err(DesignError::BadAnchor("the edge's anchor is past the outline's end"));
    }
    let j = (i + 1) % n;
    let (a, b) = (ring[i].p, ring[j].p);
    let d = b - a;
    let left = Pt::new(d.y, -d.x).unit().ok_or(DesignError::BadAnchor("the edge has no length"))?;
    // A cubic's middle moves three quarters of what both handles move.
    let off = left * (depth * 4.0 / 3.0);
    ring[i].h_out = a + d * (1.0 / 3.0) + off;
    ring[j].h_in = a + d * (2.0 / 3.0) + off;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ring(pts: &[(f64, f64)]) -> Vec<Knot> {
        pts.iter().map(|&(x, y)| Knot::corner(Pt::new(x, y))).collect()
    }

    fn near(a: Pt, b: Pt) -> bool {
        (a - b).len() < 1e-9
    }

    /// A 200 × 300 rectangle with a 40 mm wide dart in its top edge, its point at (100, 120).
    fn darted() -> Vec<Knot> {
        ring(&[(0.0, 0.0), (80.0, 0.0), (100.0, 120.0), (120.0, 0.0), (200.0, 0.0), (200.0, 150.0), (200.0, 300.0), (0.0, 300.0)])
    }

    #[test]
    fn a_dart_pivots_to_the_side_and_closes_where_it_was() {
        let r = darted();
        // The new dart at (200, 150) on the right side; the right side (fewer anchors) swings.
        let (out, new_apex) = rotate_dart(&r, 2, 5, 1.0, Side::Auto).unwrap();
        assert_eq!(out.len(), r.len());
        // The old dart is closed: its first leg is followed by the swung top edge, whose far end
        // keeps its distance from the leg.
        assert!(near(out[0].p, Pt::new(80.0, 0.0)));
        assert!(((out[1].p - out[0].p).len() - 80.0).abs() < 1e-9);
        // The new dart: swung side point, the dart point, the side point where it was.
        let apex = out.iter().position(|k| near(k.p, Pt::new(100.0, 120.0))).unwrap();
        assert_eq!(apex, new_apex);
        let (l1, l2) = (out[apex - 1].p, out[apex + 1].p);
        assert!(near(l2, Pt::new(200.0, 150.0)));
        // Both legs are as long, and the dart opens by the angle the old one had.
        let pivot = Pt::new(100.0, 120.0);
        assert!(((l1 - pivot).len() - (l2 - pivot).len()).abs() < 1e-9);
        let old = turn(pivot, Pt::new(120.0, 0.0), Pt::new(80.0, 0.0)).abs();
        assert!((turn(pivot, l2, l1).abs() - old).abs() < 1e-9);
        // Everything not swung stays put.
        for p in [(200.0, 300.0), (0.0, 300.0), (0.0, 0.0)] {
            assert!(out.iter().any(|k| near(k.p, Pt::new(p.0, p.1))));
        }
    }

    #[test]
    fn either_side_can_swing_and_part_of_a_dart_can_stay() {
        let r = darted();
        let (before, _) = rotate_dart(&r, 2, 5, 1.0, Side::Before).unwrap();
        // The left side swung: the right edge and the top right corner stay.
        assert!(near(before[0].p, Pt::new(120.0, 0.0)));
        assert!(before.iter().any(|k| near(k.p, Pt::new(200.0, 0.0))));
        assert!(!before.iter().any(|k| near(k.p, Pt::new(0.0, 300.0))));
        // Half of it: both darts open, the old one half as wide in angle.
        let (half, new_apex) = rotate_dart(&r, 2, 5, 0.5, Side::Auto).unwrap();
        assert_eq!(new_apex, 5);
        assert_eq!(half.len(), r.len() + 2);
        let pivot = Pt::new(100.0, 120.0);
        assert_eq!(half.iter().filter(|k| near(k.p, pivot)).count(), 2);
        let old = turn(pivot, Pt::new(120.0, 0.0), Pt::new(80.0, 0.0)).abs();
        assert!((turn(pivot, half[2].p, half[0].p).abs() - old / 2.0).abs() < 1e-9);
        assert_eq!(rotate_dart(&r, 2, 3, 1.0, Side::Auto), Err(DesignError::BadAnchor("the new dart is on the old dart")));
        assert_eq!(rotate_dart(&r, 2, 5, 0.0, Side::Auto), Err(DesignError::BadAmount));
        assert!(rotate_dart(&r, 9, 5, 1.0, Side::Auto).is_err());
        // A straight run is no dart.
        assert!(rotate_dart(&r, 5, 1, 1.0, Side::Auto).is_err());
    }

    #[test]
    fn a_curved_side_keeps_its_handles_when_it_swings() {
        let mut r = darted();
        r[5].h_in = Pt::new(210.0, 100.0);
        r[5].h_out = Pt::new(210.0, 200.0);
        r[4].h_out = Pt::new(205.0, 50.0);
        // The new dart at anchor 6; the swung side carries anchor 5's handles round with it.
        let (out, _) = rotate_dart(&r, 2, 6, 1.0, Side::After).unwrap();
        let k = out.iter().find(|k| (k.h_out - k.p).len() > 1.0 && (k.h_in - k.p).len() > 1.0).unwrap();
        assert!(((k.h_out - k.p).len() - 50.0_f64.hypot(10.0)).abs() < 1e-9);
        // The new dart's legs are corners on the dart's side.
        let apex = out.iter().position(|k| near(k.p, Pt::new(100.0, 120.0))).unwrap();
        assert!(near(out[apex - 1].h_out, out[apex - 1].p) && near(out[apex + 1].h_in, out[apex + 1].p));
    }

    #[test]
    fn a_split_makes_two_pieces_and_a_shared_dart_point_needs_no_seam() {
        let r = ring(&[(0.0, 0.0), (100.0, 0.0), (100.0, 100.0), (0.0, 100.0)]);
        let (a, b) = split(&r, 0, 2).unwrap();
        assert_eq!((a.len(), b.len()), (3, 3));
        // Two darts sharing a point: the halves meet there.
        let pivot = (100.0, 120.0);
        let two =
            ring(&[(0.0, 0.0), (80.0, 0.0), pivot, (120.0, 0.0), (200.0, 0.0), (200.0, 300.0), (120.0, 300.0), pivot, (80.0, 300.0), (0.0, 300.0)]);
        assert_eq!(coincident(&two, 2), Some(7));
        let (l, rt) = split(&two, 2, 7).unwrap();
        assert_eq!((l.len(), rt.len()), (5, 5));
        assert!(l.iter().all(|k| k.p.x >= 100.0) && rt.iter().all(|k| k.p.x <= 100.0));
        assert!(opening(&two, 2).unwrap() < 0.4 && (opening(&two, 5).unwrap() - std::f64::consts::FRAC_PI_2).abs() < 1e-9);
        assert!(split(&r, 0, 1).is_err() && split(&r, 1, 1).is_err() && split(&r, 0, 4).is_err());
    }

    /// A 200 × 300 skirt-like panel, waist at the top: points 0 (0,0), 1 (100,0), 2 (200,0),
    /// 3 (200,300), 4 (100,300), 5 (0,300).
    fn panel() -> Vec<Knot> {
        ring(&[(0.0, 0.0), (100.0, 0.0), (200.0, 0.0), (200.0, 300.0), (100.0, 300.0), (0.0, 300.0)])
    }

    #[test]
    fn a_hinged_slash_adds_flare_at_the_hem_and_keeps_the_waist() {
        let r = panel();
        // From the hem's middle (4) up to the waist's middle (1): 4, 5, 0, 1 stay; 2, 3, 4 turn.
        let out = spread(&r, 4, 1, 60.0, Spread::Hinge).unwrap();
        assert_eq!(out.len(), 7);
        assert!(near(out[0].p, Pt::new(100.0, 300.0)) && near(out[3].p, Pt::new(100.0, 0.0)));
        // The gap at the hem is 60 wide, the moved side turned outward (right), the waist as long.
        let gap = (out[6].p - out[0].p).len();
        assert!((gap - 60.0).abs() < 1e-9, "{gap}");
        assert!(out[6].p.x > 100.0 && out[5].p.x > 200.0);
        assert!(((out[4].p - out[3].p).len() - 100.0).abs() < 1e-9);
        assert!(((out[5].p - out[4].p).len() - 300.0).abs() < 1e-9);
    }

    #[test]
    fn a_parallel_slash_adds_even_fullness() {
        let r = panel();
        let out = spread(&r, 4, 1, 40.0, Spread::Parallel).unwrap();
        assert_eq!(out.len(), 8);
        // Both ends open 40: the waist is 40 longer, the hem too.
        assert!(near(out[4].p, Pt::new(140.0, 0.0)) && near(out[7].p, Pt::new(140.0, 300.0)));
        assert!(near(out[5].p, Pt::new(240.0, 0.0)) && near(out[6].p, Pt::new(240.0, 300.0)));
        assert!(spread(&r, 4, 5, 40.0, Spread::Parallel).is_err());
        assert!(spread(&r, 4, 1, 0.0, Spread::Hinge).is_err());
        assert!(spread(&r, 4, 1, 700.0, Spread::Hinge).is_err());
        assert!(spread(&r, 4, 9, 10.0, Spread::Hinge).is_err());
    }

    #[test]
    fn a_facing_follows_its_edge_and_ends_along_the_seams_beside_it() {
        // A bodice-like outline (shoelace area positive in either orientation is handled): a
        // straight neck from (40, 0) to (100, 60), the shoulder up and left of it, the centre
        // front straight down.
        let r = ring(&[(0.0, 0.0), (40.0, 0.0), (100.0, 60.0), (100.0, 300.0), (0.0, 300.0)]);
        let f = facing(&r, 1, 2, 30.0).unwrap();
        assert_eq!(f.len(), 4);
        assert!(near(f[0].p, Pt::new(40.0, 0.0)) && near(f[1].p, Pt::new(100.0, 60.0)));
        // The inner edge is 30 inside the neck, its ends on the shoulder (y = 0) and the centre
        // front (x = 100).
        let off = 30.0 * std::f64::consts::SQRT_2;
        assert!(near(f[2].p, Pt::new(100.0, 60.0 + off)), "{:?}", f[2].p);
        assert!(near(f[3].p, Pt::new(40.0 - off, 0.0)), "{:?}", f[3].p);
        // The other way round the outline: the same facing.
        let rev: Vec<Knot> = r.iter().rev().copied().collect();
        let g = facing(&rev, 2, 3, 30.0).unwrap();
        assert!(g.iter().any(|k| near(k.p, Pt::new(100.0, 60.0 + off))) && g.iter().any(|k| near(k.p, Pt::new(40.0 - off, 0.0))));
        assert!(facing(&r, 1, 1, 30.0).is_err() && facing(&r, 1, 2, 0.0).is_err() && facing(&r, 1, 7, 30.0).is_err());
    }

    #[test]
    fn a_curved_facing_keeps_its_width() {
        // A quarter circle of radius 100 about (0, 0) as the neck: from (100, 0) to (0, 100),
        // the rest of the outline outside it.
        let k = 0.552_284_749_8 * 100.0;
        let mut r = ring(&[(100.0, 0.0), (0.0, 100.0), (0.0, 300.0), (300.0, 300.0), (300.0, 0.0)]);
        r[0].h_out = Pt::new(100.0, k);
        r[1].h_in = Pt::new(k, 100.0);
        let f = facing(&r, 0, 1, 20.0).unwrap();
        // The inner edge, in pieces, is a quarter circle of radius 120 within a twentieth of a mm.
        let inner = &f[2..];
        assert!(inner.len() >= 4);
        for w in inner.windows(2) {
            let (a, b) = (w[0], w[1]);
            for t in [0.0, 0.25, 0.5, 0.75] {
                let mt = 1.0 - t;
                let p = a.p * (mt * mt * mt) + a.h_out * (3.0 * mt * mt * t) + b.h_in * (3.0 * mt * t * t) + b.p * (t * t * t);
                assert!((p.len() - 120.0).abs() < 0.05, "{t}: {}", p.len());
            }
        }
    }

    #[test]
    fn a_knife_pleat_takes_twice_its_depth_and_folds_toward_the_side_that_stays() {
        let r = panel();
        let (out, m) = pleat(&r, 4, 1, 20.0, PleatKind::Knife, 5.0).unwrap();
        // Spread 40: the waist and the hem 40 longer.
        assert!(near(out[4].p, Pt::new(140.0, 0.0)) && near(out[7].p, Pt::new(140.0, 300.0)));
        // The placement line is the slash, the fold line the moved edge, 40 to the right.
        assert_eq!(m.placements, vec![[Pt::new(100.0, 300.0), Pt::new(100.0, 0.0)]]);
        assert!(m.folds.len() == 1 && near(m.folds[0][0], Pt::new(140.0, 300.0)) && near(m.folds[0][1], Pt::new(140.0, 0.0)));
        // The arrow points from the fold to the placement line, at mid height.
        let [shaft, head] = &m.arrows[0];
        assert!(shaft[0].x > shaft[1].x && near(shaft[1], Pt::new(104.0, 150.0)));
        assert!(near(head[1], shaft[1]) && head[0].x > head[1].x && head[2].x > head[1].x);
        assert!(pleat(&r, 4, 1, 0.0, PleatKind::Knife, 5.0).is_err());
    }

    #[test]
    fn an_inverted_pleat_takes_four_times_its_depth_and_folds_both_edges_to_the_middle() {
        let r = panel();
        let (out, m) = pleat(&r, 4, 1, 20.0, PleatKind::Inverted, 5.0).unwrap();
        // Spread 80; both edges fold, to the middle 40 in.
        assert!(near(out[4].p, Pt::new(180.0, 0.0)) && near(out[5].p, Pt::new(280.0, 0.0)));
        assert_eq!(m.folds.len(), 2);
        assert!(near(m.folds[0][0], Pt::new(100.0, 300.0)) && near(m.folds[1][0], Pt::new(180.0, 300.0)));
        assert!(m.placements.len() == 1 && near(m.placements[0][0], Pt::new(140.0, 300.0)) && near(m.placements[0][1], Pt::new(140.0, 0.0)));
        // The arrows point toward each other, both ending short of the middle.
        let (a, b) = (&m.arrows[0][0], &m.arrows[1][0]);
        assert!(a[1].x > a[0].x && b[1].x < b[0].x && a[1].x < 140.0 && b[1].x > 140.0);
    }

    #[test]
    fn a_box_pleat_folds_out_past_both_edges_of_its_spread() {
        let r = panel();
        let (out, m) = pleat(&r, 4, 1, 20.0, PleatKind::Box, 5.0).unwrap();
        // Spread 80, like an inverted pleat; the folds 20 in from its edges (100 and 180), each
        // brought 20 out past its edge.
        assert!(near(out[4].p, Pt::new(180.0, 0.0)));
        let xs = |l: &[[Pt; 2]]| l.iter().map(|l| l[0].x).collect::<Vec<_>>();
        assert!(xs(&m.folds).iter().zip([120.0, 160.0]).all(|(a, b)| (a - b).abs() < 1e-9), "{:?}", m.folds);
        assert!(xs(&m.placements).iter().zip([80.0, 200.0]).all(|(a, b)| (a - b).abs() < 1e-9), "{:?}", m.placements);
        assert!(m.folds.iter().chain(&m.placements).all(|l| (l[0].y - 300.0).abs() < 1e-9 && l[1].y.abs() < 1e-9));
        // The arrows point away from each other, out to the placements.
        let (a, b) = (&m.arrows[0][0], &m.arrows[1][0]);
        assert!(a[1].x < a[0].x && b[1].x > b[0].x && a[1].x > 80.0 && b[1].x < 200.0);
    }

    #[test]
    fn a_gathering_mark_waves_inside_its_edge_between_two_ticks() {
        // The panel's waist, from (0, 0) to (200, 0), the panel below it (y down).
        let r = panel();
        let m = gather_marks(&r, 0, 2, 5.0, 8.0).unwrap();
        assert_eq!(m.len(), 3);
        // The wave keeps inside the panel, 5 ± 2 below the waist, over the whole edge.
        let wave = &m[0];
        assert!(wave.iter().all(|p| p.y >= 3.0 - 1e-9 && p.y <= 7.0 + 1e-9), "{wave:?}");
        assert!((wave[0].x).abs() < 1e-9 && (wave[wave.len() - 1].x - 200.0).abs() < 1e-9);
        assert!(wave.iter().any(|p| p.y > 6.9) && wave.iter().any(|p| p.y < 3.1));
        // The ticks cross the edge at both ends.
        assert!(near(m[1][0], Pt::new(0.0, -5.0)) && near(m[1][1], Pt::new(0.0, 10.0)));
        assert!(near(m[2][0], Pt::new(200.0, -5.0)));
        assert!((run_length(&r, 0, 2) - 200.0).abs() < 1e-9 && (run_length(&r, 2, 0) - 800.0).abs() < 1e-9);
        assert!(gather_marks(&r, 1, 1, 5.0, 8.0).is_err() && gather_marks(&r, 0, 2, 0.0, 8.0).is_err());
    }

    #[test]
    fn a_flat_collar_joins_the_necklines_at_the_neck_point_with_the_shoulders_overlapping() {
        // A back (CB on the left, neck point at (50, 0), shoulder down to (150, 20)) and a front
        // drafted elsewhere (neck point (350, 0), shoulder down to (250, 20), CF at (400, 60)).
        let back = ring(&[(0.0, 10.0), (50.0, 0.0), (150.0, 20.0), (150.0, 300.0), (0.0, 300.0)]);
        let front = ring(&[(350.0, 0.0), (250.0, 20.0), (250.0, 300.0), (400.0, 300.0), (400.0, 60.0)]);
        let b = Neckline { ring: &back, from: 0, to: 1, shoulder: 2 };
        // The front's outline runs from its neck point (4) forward to centre front (0).
        let rev: Vec<Knot> = front.iter().rev().copied().collect();
        let f_rev = Neckline { ring: &rev, from: 4, to: 0, shoulder: 3 };
        let c = flat_collar(b, f_rev, 30.0, 60.0, 0.0).unwrap();
        // The neck edge starts at centre back and passes the neck point; it is as long as both
        // necklines.
        assert!(near(c.outline[0].p, Pt::new(0.0, 10.0)) && near(c.outline[1].p, Pt::new(50.0, 0.0)));
        let (lb, lf) = ((Pt::new(50.0, 0.0) - Pt::new(0.0, 10.0)).len(), (Pt::new(400.0, 60.0) - Pt::new(350.0, 0.0)).len());
        assert!((c.neck_length - (lb + lf)).abs() < 1e-6, "{} {}", c.neck_length, lb + lf);
        // The front's shoulder point lies 30 round the turn over the back's shoulder line.
        let lshoulder = (Pt::new(250.0, 20.0) - Pt::new(350.0, 0.0)).len();
        let fs = c.front_shoulder - Pt::new(50.0, 0.0);
        assert!((fs.len() - lshoulder).abs() < 1e-9);
        let a = turn(Pt::new(0.0, 0.0), Pt::new(100.0, 20.0), fs).abs();
        assert!((a * lshoulder - 30.0).abs() < 1e-6, "{}", a * lshoulder);
        // The outer edge is 60 from the neckline at centre back (a straight centre back line).
        let last = c.outline[c.outline.len() - 1].p;
        assert!(((last - Pt::new(0.0, 10.0)).len() - 60.0).abs() < 1e-6);
        // The outer edge keeps the width from the neckline all round, past the corner at the
        // neck point too (no spike where the two necklines meet at an angle).
        let neck_pts = [Pt::new(0.0, 10.0), Pt::new(50.0, 0.0), c.outline[2].p];
        let dist = |q: Pt| {
            neck_pts
                .windows(2)
                .map(|s| {
                    let (a, b) = (s[0], s[1]);
                    let t = ((q - a).dot(b - a) / (b - a).dot(b - a)).clamp(0.0, 1.0);
                    (q - (a + (b - a) * t)).len()
                })
                .fold(f64::MAX, f64::min)
        };
        let outer_of = |c: &Collar, from: usize| c.outline[from..].iter().map(|k| dist(k.p)).collect::<Vec<f64>>();
        assert!(outer_of(&c, 4).iter().all(|d| (d - 60.0).abs() < 0.5), "{:?}", outer_of(&c, 4));
        // Lowered 10: the outer edge 70 from the neckline as drawn, all round (an edge offset
        // from the lowered one would spike out at the corner), and the neck edge 10 below it at
        // centre back.
        let lowered = flat_collar(b, f_rev, 30.0, 60.0, 10.0).unwrap();
        let start = lowered.outline.iter().position(|k| (dist(k.p) - 70.0).abs() < 0.5).unwrap_or(0);
        assert!(start > 0 && outer_of(&lowered, start).iter().all(|d| (d - 70.0).abs() < 0.5), "{:?}", outer_of(&lowered, start));
        assert!(((lowered.outline[0].p - Pt::new(0.0, 10.0)).len() - 10.0).abs() < 1e-6);
        assert!(flat_collar(b, f_rev, 30.0, 0.0, 0.0).is_err());
        assert!(flat_collar(Neckline { shoulder: 9, ..b }, f_rev, 30.0, 60.0, 0.0).is_err());
    }

    #[test]
    fn a_patch_pocket_is_its_depth_and_hem_tall_with_the_corners_asked_for() {
        let p = patch_pocket(140.0, 160.0, 30.0, PocketCorners::Square).unwrap();
        assert_eq!(
            p.piece.iter().map(|k| k.p).collect::<Vec<_>>(),
            [Pt::new(0.0, 0.0), Pt::new(140.0, 0.0), Pt::new(140.0, 190.0), Pt::new(0.0, 190.0)]
        );
        assert_eq!(p.fold, [Pt::new(0.0, 30.0), Pt::new(140.0, 30.0)]);
        assert_eq!(p.finished[2].p, Pt::new(140.0, 160.0));
        // Rounded: the bottom is the radius in from each side, and as long as a quarter circle
        // each corner (within a hair of the cubic's).
        let r = patch_pocket(140.0, 160.0, 30.0, PocketCorners::Round(20.0)).unwrap();
        assert_eq!(r.piece.len(), 6);
        assert!(near(r.piece[3].p, Pt::new(120.0, 190.0)) && near(r.piece[4].p, Pt::new(20.0, 190.0)));
        let round = run_length(&r.piece, 2, 3);
        assert!((round - std::f64::consts::FRAC_PI_2 * 20.0).abs() < 0.02, "{round}");
        // Pointed: the bottom middle 25 below the sides' ends.
        let v = patch_pocket(140.0, 160.0, 30.0, PocketCorners::Point(25.0)).unwrap();
        assert!(near(v.piece[3].p, Pt::new(70.0, 190.0)) && near(v.piece[2].p, Pt::new(140.0, 165.0)));
        for bad in [
            patch_pocket(0.0, 160.0, 30.0, PocketCorners::Square),
            patch_pocket(140.0, 160.0, -1.0, PocketCorners::Square),
            patch_pocket(140.0, 160.0, 30.0, PocketCorners::Round(80.0)),
            patch_pocket(140.0, 160.0, 30.0, PocketCorners::Point(200.0)),
        ] {
            assert_eq!(bad, Err(DesignError::BadAmount));
        }
    }

    #[test]
    fn smoothing_mirror_pieces_across_their_seam_squares_the_curve_to_it() {
        // A: the seam along the top from (0, 0) to the meeting point (100, 0), the curve (a hem
        // or armhole) leaving it down and out to (160, 100). B is A's mirror image, drafted
        // elsewhere.
        let mut a = ring(&[(0.0, 0.0), (100.0, 0.0), (160.0, 100.0), (0.0, 100.0)]);
        a[1].h_out = Pt::new(120.0, 40.0);
        let mirror = |k: &Knot| Knot {
            p: Pt::new(600.0 - k.p.x, k.p.y),
            h_in: Pt::new(600.0 - k.h_out.x, k.h_out.y),
            h_out: Pt::new(600.0 - k.h_in.x, k.h_in.y),
        };
        let b: Vec<Knot> = a.iter().rev().map(mirror).collect();
        // In b (reversed), the meeting point is 2 and the seam's other end 3.
        assert!(near(b[2].p, Pt::new(500.0, 0.0)) && near(b[3].p, Pt::new(600.0, 0.0)));
        let ja = Junction { ring: &a, at: 1, seam: 0 };
        let jb = Junction { ring: &b, at: 2, seam: 3 };
        let (ka, kb, corner) = smooth_across(ja, jb).unwrap();
        // There was a corner (each curve 26.6° off square, so 53° together); now both leave the
        // seam square, straight down, their handles as long as before.
        assert!((corner - 2.0 * (20.0_f64).atan2(40.0)).abs() < 1e-9, "{corner}");
        let len = (Pt::new(20.0, 40.0)).len();
        assert!(near(ka.h_out, Pt::new(100.0, len)) && near(kb.h_in, Pt::new(500.0, len)), "{ka:?} {kb:?}");
        // Smoothing again finds no corner.
        let mut a2 = a.clone();
        a2[1] = ka;
        let mut b2 = b.clone();
        b2[2] = kb;
        let (_, _, again) = smooth_across(Junction { ring: &a2, ..ja }, Junction { ring: &b2, ..jb }).unwrap();
        assert!(again.abs() < 1e-9);
        assert!(smooth_across(Junction { seam: 3, ..ja }, jb).is_err());
    }

    #[test]
    fn a_stand_collar_is_as_long_as_the_neckline_and_rises_at_centre_front() {
        // The Creative Curator's example: 90 back, 105 front, 15 extension, 35 high, 15 rise.
        let c = stand_collar(90.0, 105.0, 35.0, 15.0, 15.0).unwrap();
        let o = &c.outline;
        // The neck edge's four points, then the top edge (the offset front curve may be in pieces).
        assert!(o.len() >= 8);
        // The neck edge: straight to the shoulder, the front curve 105 long, rising 15, then
        // the extension 15 on.
        assert!(near(o[0].p, Pt::new(0.0, 0.0)) && near(o[1].p, Pt::new(90.0, 0.0)) && near(c.shoulder, o[1].p));
        assert!((run_length(o, 1, 2) - 105.0).abs() < 1e-6, "{}", run_length(o, 1, 2));
        assert!((o[2].p.y + 15.0).abs() < 1e-9 && near(c.centre_front[0], o[2].p));
        assert!((run_length(o, 2, 3) - 15.0).abs() < 1e-9);
        // The top edge 35 above all the way: at centre back, at centre front, at the front end.
        assert!(near(o[o.len() - 1].p, Pt::new(0.0, -35.0)), "{:?}", o[o.len() - 1].p);
        assert!(((c.centre_front[1] - c.centre_front[0]).len() - 35.0).abs() < 1e-6);
        assert!(((o[4].p - o[3].p).len() - 35.0).abs() < 1e-6);
        // The top edge, curving round the neck, is shorter than the neck edge.
        assert!(c.top < 195.0 && c.top > 180.0, "{}", c.top);
        // Square to the neck edge at the front end.
        assert!((o[4].p - o[3].p).dot(o[3].p - o[2].p).abs() < 1e-6);
        // Edge to edge: no extension, the front end at centre front.
        assert_eq!(stand_collar(90.0, 105.0, 35.0, 15.0, 0.0).unwrap().outline.len(), o.len() - 2);
        assert!(stand_collar(90.0, 105.0, 0.0, 15.0, 0.0).is_err() && stand_collar(90.0, 10.0, 35.0, 15.0, 0.0).is_err());
    }

    #[test]
    fn an_in_seam_pocket_bag_hangs_inside_the_piece_from_its_seam() {
        // A panel 200 wide and 600 long, its side seam on the right from the waist (1) down.
        let r = ring(&[(0.0, 0.0), (200.0, 0.0), (200.0, 600.0), (0.0, 600.0)]);
        for (rr, from, to) in [(r.clone(), 1, 2), (reversed(&r), 2, 1)] {
            let p = in_seam_pocket(&rr, from, to, 90.0, 140.0, 150.0, 130.0, 15.0).unwrap();
            assert_eq!(p.opening, [90.0, 230.0]);
            let bag = &p.bag;
            // The seam edge from 15 above the opening to 15 below it.
            assert!(near(bag[0].p, Pt::new(200.0, 75.0)) && near(bag[16].p, Pt::new(200.0, 245.0)), "{:?} {:?}", bag[0].p, bag[16].p);
            assert!(bag[..17].iter().all(|k| (k.p.x - 200.0).abs() < 1e-9));
            // In from the seam: the tabs 15 in, the widest 130 in, the bottom 150 below the
            // opening; everything inside the panel.
            assert!(near(bag[17].p, Pt::new(185.0, 245.0)) && near(bag[20].p, Pt::new(185.0, 75.0)));
            assert!(near(bag[18].p, Pt::new(200.0 - 78.0, 380.0)) && near(bag[19].p, Pt::new(70.0, 125.0)));
            for w in bag[16..].windows(2) {
                for k in 0..=8 {
                    let q = cubic_at(w[0].p, w[0].h_out, w[1].h_in, w[1].p, k as f64 / 8.0).0;
                    assert!(q.x >= 0.0 && q.x <= 200.0 + 1e-9 && q.y > 0.0 && q.y < 600.0, "{q:?}");
                }
            }
        }
        assert!(in_seam_pocket(&r, 1, 2, 500.0, 140.0, 150.0, 130.0, 15.0).is_err());
        assert!(in_seam_pocket(&r, 1, 2, 90.0, 140.0, 150.0, 10.0, 15.0).is_err());
        assert!(in_seam_pocket(&r, 1, 1, 90.0, 140.0, 150.0, 130.0, 15.0).is_err());
    }

    #[test]
    fn a_welt_pockets_pieces_from_its_opening() {
        // The Shapes of Fabric's example: a 120 opening, 7 welts, 7 and 15 allowances.
        let p = welt_pocket(120.0, 7.0, 150.0, Welts::Double, 7.0, 15.0).unwrap();
        let size = |r: &[Knot]| r[2].p - r[0].p;
        assert!(near(size(&p.welt), Pt::new(150.0, 28.0)) && (p.fold[0].y - 14.0).abs() < 1e-9);
        // The bags as long as the welt; the upper deeper by the opening's height (two welts).
        assert!(near(size(&p.bags[0]), Pt::new(150.0, 157.0)) && near(size(&p.bags[1]), Pt::new(150.0, 171.0)));
        assert!(near(size(&p.opening), Pt::new(120.0, 14.0)));
        // The cut along the middle, 7 short of each end, then to the corners.
        assert!(near(p.cuts[0][0], Pt::new(7.0, 7.0)) && near(p.cuts[0][1], Pt::new(113.0, 7.0)) && p.cuts.len() == 5);
        // A single welt fills the opening alone.
        let s = welt_pocket(120.0, 15.0, 150.0, Welts::Single, 7.0, 15.0).unwrap();
        assert!(near(size(&s.opening), Pt::new(120.0, 15.0)) && near(size(&s.welt), Pt::new(150.0, 44.0)));
        // No piece overlaps another.
        assert!(s.bags[0][0].p.y > s.welt[2].p.y && s.bags[1][0].p.x > s.bags[0][1].p.x);
        assert!(
            welt_pocket(20.0, 15.0, 150.0, Welts::Double, 7.0, 15.0).is_err() && welt_pocket(120.0, 0.0, 150.0, Welts::Single, 7.0, 15.0).is_err()
        );
    }

    #[test]
    fn a_collar_fall_fits_its_stand_and_turns_down_over_it() {
        // NIMI's numbers in mm: 38 high, the point 51 long and 19 out, 13 down at the front.
        let f = collar_fall(200.0, 38.0, 13.0, 19.0, 51.0).unwrap();
        assert_eq!(f.len(), 4);
        // The neck edge as long as the stand's top edge, ending 13 down.
        assert!((run_length(&f, 0, 1) - 200.0).abs() < 1e-6 && (f[1].p.y - 13.0).abs() < 1e-9);
        // The point 19 past the neck edge's end and 51 up from it; centre back 38 high.
        assert!(near(f[2].p, f[1].p + Pt::new(19.0, -51.0)) && near(f[3].p, Pt::new(0.0, -38.0)));
        assert!(collar_fall(200.0, 0.0, 13.0, 19.0, 51.0).is_err() && collar_fall(20.0, 38.0, 13.0, 19.0, 51.0).is_err());
    }

    #[test]
    fn a_half_opens_out_across_its_fold_and_an_edge_bows() {
        // A half front: centre front on the right, from (100, 0) down to (100, 200).
        let half = [Pt::new(100.0, 0.0), Pt::new(100.0, 200.0), Pt::new(0.0, 200.0), Pt::new(0.0, 40.0), Pt::new(60.0, 0.0)].map(Knot::corner);
        let whole = opened_out(&half, 0, 1).unwrap();
        let pts: Vec<Pt> = whole.iter().map(|k| k.p).collect();
        assert_eq!(
            pts,
            [
                Pt::new(100.0, 200.0),
                Pt::new(0.0, 200.0),
                Pt::new(0.0, 40.0),
                Pt::new(60.0, 0.0),
                Pt::new(100.0, 0.0),
                Pt::new(140.0, 0.0),
                Pt::new(200.0, 40.0),
                Pt::new(200.0, 200.0),
            ]
        );
        // The same, the fold edge named the other way round, and a curved knot keeps its curve.
        assert_eq!(opened_out(&half, 1, 0).unwrap(), whole);
        let mut curved = half;
        curved[4].h_in = Pt::new(80.0, 0.0);
        curved[4].h_out = Pt::new(50.0, 5.0);
        let w = opened_out(&curved, 0, 1).unwrap();
        assert_eq!((w[5].h_in, w[5].h_out), (Pt::new(150.0, 5.0), Pt::new(120.0, 0.0)));
        assert!(opened_out(&half, 0, 2).is_err() && opened_out(&half, 0, 9).is_err());
        // A straight edge bowed 10 to its left: its middle moves 10.
        let mut ring = [Pt::new(0.0, 0.0), Pt::new(100.0, 0.0), Pt::new(50.0, 80.0)].map(Knot::corner);
        bow(&mut ring, 0, 10.0).unwrap();
        let mid = (ring[0].p + ring[0].h_out * 3.0 + ring[1].h_in * 3.0 + ring[1].p) * 0.125;
        assert!((mid - Pt::new(50.0, -10.0)).len() < 1e-9, "{mid:?}");
    }

    #[test]
    fn a_dart_is_shortened_along_its_centre_line_and_the_seam_stays() {
        // A rectangle with a dart 40 wide and 100 deep in its top edge, its point at (50, 100).
        let pts = [(0.0, 0.0), (30.0, 0.0), (50.0, 100.0), (70.0, 0.0), (100.0, 0.0), (100.0, 200.0), (0.0, 200.0)];
        let ring: Vec<Knot> = pts.iter().map(|&(x, y)| Knot::corner(Pt::new(x, y))).collect();
        let out = shorten_dart(&ring, 2, 25.0).unwrap();
        assert!((out[2].p - Pt::new(50.0, 75.0)).len() < 1e-9, "{:?}", out[2].p);
        assert_eq!((out[1], out[3]), (ring[1], ring[3]), "the legs' ends stay");
        // Lengthened back.
        let back = shorten_dart(&out, 2, -25.0).unwrap();
        assert!((back[2].p - ring[2].p).len() < 1e-9);
        // Not past its opening, not a corner, not a curved leg.
        assert!(shorten_dart(&ring, 2, 100.0).is_err());
        assert!(shorten_dart(&ring, 5, 10.0).is_err(), "a right angle is no dart");
        let mut curved = ring.clone();
        curved[2].h_in = Pt::new(45.0, 60.0);
        assert!(shorten_dart(&curved, 2, 10.0).is_err());
    }

    #[test]
    fn a_darts_legs_are_curved_out_or_in_and_its_ends_stay() {
        // The dart of the shortening test: 40 wide, its point at (50, 100).
        let pts = [(0.0, 0.0), (30.0, 0.0), (50.0, 100.0), (70.0, 0.0), (100.0, 0.0), (100.0, 200.0), (0.0, 200.0)];
        let ring: Vec<Knot> = pts.iter().map(|&(x, y)| Knot::corner(Pt::new(x, y))).collect();
        let mid = |r: &[Knot], i: usize| (r[i].p + r[i].h_out * 3.0 + r[i + 1].h_in * 3.0 + r[i + 1].p) * 0.125;
        let convex = curve_dart(&ring, 2, 3.0).unwrap();
        let (l, r) = (mid(&convex, 1), mid(&convex, 2));
        // Each leg's middle 3 off the straight leg's, away from the centre line (x = 50).
        assert!(((l - Pt::new(40.0, 50.0)).len() - 3.0).abs() < 1e-9 && l.x < 40.0, "{l:?}");
        assert!(((r - Pt::new(60.0, 50.0)).len() - 3.0).abs() < 1e-9 && r.x > 60.0, "{r:?}");
        assert!((1..4).all(|i| convex[i].p == ring[i].p), "the ends and the point stay");
        let concave = curve_dart(&ring, 2, -3.0).unwrap();
        assert!(mid(&concave, 1).x > 40.0 && mid(&concave, 2).x < 60.0);
        // Not so far the legs cross, not twice (the legs are curved), not a corner.
        assert!(curve_dart(&ring, 2, 10.0).is_err() && curve_dart(&ring, 2, -10.0).is_err());
        assert!(curve_dart(&convex, 2, 3.0).is_err());
        assert!(curve_dart(&ring, 5, 3.0).is_err(), "a right angle is no dart");
    }

    #[test]
    fn a_pocket_flap_is_its_shape_with_the_allowance_all_round() {
        let bounds = |ks: &[Knot]| {
            ks.iter().fold((f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY), |(a, b, c, d), k| {
                (a.min(k.p.x), b.min(k.p.y), c.max(k.p.x), d.max(k.p.y))
            })
        };
        for shape in [FlapShape::Square, FlapShape::Rounded, FlapShape::Pointed] {
            let (cut, sew) = pocket_flap(118.0, 50.0, shape, 7.0).unwrap();
            // The stitching line 118 × 50 at (7, 7); the cutting line 7 out all round.
            let (x0, y0, x1, y1) = bounds(&sew);
            assert!((x0 - 7.0).abs() < 1e-9 && (y0 - 7.0).abs() < 1e-9 && (x1 - 125.0).abs() < 1e-9 && (y1 - 57.0).abs() < 1e-9, "{shape:?}");
            let (x0, y0, x1, y1) = bounds(&cut);
            // A point's mitre reaches a little further than the allowance below it.
            let low = if shape == FlapShape::Pointed { y1 > 64.0 && y1 < 65.0 } else { (y1 - 64.0).abs() < 1e-9 };
            assert!(x0.abs() < 1e-9 && y0.abs() < 1e-9 && (x1 - 132.0).abs() < 1e-9 && low, "{shape:?} {:?}", bounds(&cut));
        }
        // The point: the middle of the lower edge, straight below it on the cutting line.
        let (cut, sew) = pocket_flap(118.0, 50.0, FlapShape::Pointed, 7.0).unwrap();
        assert_eq!((sew.len(), cut[3].p.x), (5, 66.0));
        assert_eq!(FlapShape::parse("rounded"), Some(FlapShape::Rounded));
        assert!(pocket_flap(0.0, 50.0, FlapShape::Square, 7.0).is_err() && pocket_flap(50.0, 150.0, FlapShape::Square, 7.0).is_err());
    }
}
