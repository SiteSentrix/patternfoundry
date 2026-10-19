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
//! the far end (flare: the gap closes to nothing there) or parallel (even fullness, a pleat).
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
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    /// The side with fewer anchors (the forward side on a tie).
    Auto,
    /// From the dart's second leg forward to the new dart.
    After,
    /// From the new dart forward to the dart's first leg.
    Before,
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
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
}
