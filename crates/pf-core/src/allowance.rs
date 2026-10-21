//! The cutting line: a piece's seam line pushed outward by each edge's seam allowance.
//!
//! The seam line arrives flattened, as an [`Outline`]: a closed polyline whose segments each know
//! the edge of the piece they belong to. Each segment's allowance line is the segment moved
//! outward by its edge's width; neighbouring allowance lines are joined where they cross. Where two
//! edges meet (a corner) the piece's [`Corner`] style decides the join; inside a curved edge the
//! joins are mitres, cut off (bevelled) where a mitre would reach far out.
//!
//! A dart (a sharp inward V of two straight legs) is not cut round: the cutting line crosses it
//! with the dart's cap, drawn as a pattern maker does by folding the dart closed, pressed toward
//! the side before it, drawing the allowance, and opening it again: the allowance lines meet the
//! legs where the folded line crosses them, and peak over the dart's middle line where the
//! folded-under dart reaches.
//!
//! Known limit: a seam line whose inward curves are tighter than the allowance (a deep, narrow
//! notch in the outline) gives a cutting line that crosses itself there.

use crate::Pt;
use crate::piece::Corner;

/// A mitre inside a curved edge reaches at most this many allowance widths from the seam line.
const CURVE_MITRE_LIMIT: f64 = 4.0;
/// A mitred corner reaches at most this many allowance widths from the corner (as SVG's default
/// mitre limit); past it the corner is cut flat.
const CORNER_MITRE_LIMIT: f64 = 4.0;
/// Points closer than this (in the outline's unit) are one point.
const EPS: f64 = 1e-9;
/// Most points an outline may have (a bound for untrusted input).
pub const MAX_POINTS: usize = 1_000_000;

/// A closed seam line, flattened: `points[i]` to `points[i + 1]` (the last back to the first) is
/// segment `i`, part of edge `edge_of[i]`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Outline {
    pub points: Vec<Pt>,
    pub edge_of: Vec<usize>,
}

/// Why an outline gives no cutting line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OutlineError {
    /// Fewer than three distinct points: not an area.
    TooFewPoints,
    /// A coordinate that isn't a number.
    NotFinite,
    /// No area (all points on a line).
    NoArea,
    /// `edge_of` doesn't have one entry per point, or there are more than [`MAX_POINTS`].
    Malformed,
}

impl std::fmt::Display for OutlineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            OutlineError::TooFewPoints => "a pattern piece needs a closed seam line of at least three points",
            OutlineError::NotFinite => "the seam line has a point that isn't a number",
            OutlineError::NoArea => "the seam line encloses no area",
            OutlineError::Malformed => "the seam line is malformed",
        })
    }
}

impl std::error::Error for OutlineError {}

/// One segment of a cleaned outline.
#[derive(Clone, Copy, Debug)]
struct Seg {
    start: Pt,
    end: Pt,
    edge: usize,
    /// Unit direction.
    dir: Pt,
    /// Unit outward normal.
    out: Pt,
}

impl Outline {
    /// The outline without zero-length segments, as segments with their directions and outward
    /// normals.
    fn segments(&self) -> Result<Vec<Seg>, OutlineError> {
        let n = self.points.len();
        if n != self.edge_of.len() || n > MAX_POINTS {
            return Err(OutlineError::Malformed);
        }
        if self.points.iter().any(|p| !p.is_finite()) {
            return Err(OutlineError::NotFinite);
        }
        // Keep the start of every segment that has a length.
        let mut kept: Vec<(Pt, usize)> = Vec::with_capacity(n);
        for i in 0..n {
            let (Some(&a), Some(&b), Some(&e)) = (self.points.get(i), self.points.get((i + 1) % n), self.edge_of.get(i)) else {
                return Err(OutlineError::Malformed);
            };
            if (b - a).len() > EPS {
                kept.push((a, e));
            }
        }
        if kept.len() < 3 {
            return Err(OutlineError::TooFewPoints);
        }
        let m = kept.len();
        let area: f64 = (0..m).map(|i| kept[i].0.cross(kept[(i + 1) % m].0)).sum::<f64>() / 2.0;
        if !area.is_finite() || area.abs() < EPS {
            return Err(OutlineError::NoArea);
        }
        let side = area.signum();
        let mut segs = Vec::with_capacity(m);
        for i in 0..m {
            let (start, edge) = kept[i];
            let end = kept[(i + 1) % m].0;
            // Dropping a zero-length segment can leave two kept points on top of each other only
            // when three or more coincide; skip what has no direction.
            let Some(dir) = (end - start).unit() else {
                continue;
            };
            segs.push(Seg { start, end, edge, dir, out: Pt::new(dir.y, -dir.x) * side });
        }
        if segs.len() < 3 {
            return Err(OutlineError::TooFewPoints);
        }
        Ok(segs)
    }

    /// The edges in drawing order (each edge once).
    pub fn edges(&self) -> Vec<usize> {
        let mut out: Vec<usize> = Vec::new();
        for &e in &self.edge_of {
            if out.last() != Some(&e) && !out.contains(&e) {
                out.push(e);
            }
        }
        out
    }

    /// The point `at` (0 to 1) of the way along `edge` by length, with the seam line's direction
    /// and outward normal there. `None` when the outline has no such edge or no area.
    pub fn point_on_edge(&self, edge: usize, at: f64) -> Option<(Pt, Pt, Pt)> {
        let segs = self.segments().ok()?;
        // The edge's segments in order, starting after any segment of another edge (an edge can
        // wrap round the end of the list when the outline starts inside it).
        let first = segs.iter().position(|s| s.edge != edge).unwrap_or(0);
        let ordered: Vec<&Seg> = segs.iter().cycle().skip(first).take(segs.len()).filter(|s| s.edge == edge).collect();
        let total: f64 = ordered.iter().map(|s| (s.end - s.start).len()).sum();
        if ordered.is_empty() || total <= EPS {
            return None;
        }
        let mut left = at.clamp(0.0, 1.0) * total;
        for s in &ordered {
            let len = (s.end - s.start).len();
            if left <= len {
                return Some((s.start + s.dir * left, s.dir, s.out));
            }
            left -= len;
        }
        let s = ordered.last()?;
        Some((s.end, s.dir, s.out))
    }
}

/// A dart is a sharp inward corner of two straight legs: legs closer than this (degrees).
const DART_DEGREES: f64 = 50.0;

/// `p` turned by `angle` (radians) about `about`.
fn turn_about(p: Pt, about: Pt, angle: f64) -> Pt {
    let (s, c) = angle.sin_cos();
    let d = p - about;
    about + Pt::new(d.x * c - d.y * s, d.x * s + d.y * c)
}

/// `p` reflected across the line through `on` along the unit `dir`.
fn reflect(p: Pt, on: Pt, dir: Pt) -> Pt {
    let d = p - on;
    on + dir * (2.0 * d.dot(dir)) - d
}

/// The cutting line across a dart: `e1` the segment before it, `leg1` and `leg2` its legs (the
/// apex is where they meet), `e2` the segment after it, with the allowances `w1` and `w2` of the
/// segments either side. The points from where the line before the dart meets its first leg to
/// where the line after it continues; `None` when the lines don't cross (it is then cut round).
fn dart_cap(e1: &Seg, leg1: &Seg, leg2: &Seg, e2: &Seg, w1: f64, w2: f64) -> Option<Vec<Pt>> {
    let (a, apex, b) = (leg1.start, leg1.end, leg2.end);
    let u = (a - apex).unit()?;
    let v = (b - apex).unit()?;
    // Closing the dart turns everything after it about the apex until `b` is on `a`.
    let close = u.y.atan2(u.x) - v.y.atan2(v.x);
    let l1 = (a + e1.out * w1, e1.dir);
    let l2 = (turn_about(b + e2.out * w2, apex, close), turn_about(apex + e2.dir, apex, close) - apex);
    // The dart's side of the first leg (where its gap and the far side are).
    let dart_side = u.cross(b - apex).signum();
    let on_dart_side = |p: Pt| u.cross(p - apex).signum() == dart_side;
    let q = cross_point(l1.0, l1.1, l2.0, l2.1);
    // Where the folded allowance line crosses the leg: on the line after the dart when the two
    // meet before the leg, else on the line before it.
    let (mut points, across, extra) = match q {
        Some(q) if !on_dart_side(q) => {
            let q1 = cross_point(l2.0, l2.1, apex, u)?;
            (vec![q, q1], l2, None)
        }
        Some(q) => (vec![cross_point(l1.0, l1.1, apex, u)?], l1, Some(q)),
        None => (vec![cross_point(l1.0, l1.1, apex, u)?], l1, None),
    };
    let q1 = *points.last()?;
    // The dart, folded toward the first side, lies over it mirrored in the first leg: its middle
    // line reaches the allowance line where that mirror crosses it.
    let middle = (u + v).unit()?;
    let mirrored = reflect(apex + middle, apex, u) - apex;
    let c = reflect(cross_point(apex, mirrored, across.0, across.1)?, apex, u);
    points.push(c);
    points.push(turn_about(q1, apex, -close));
    if let Some(q) = extra {
        points.push(turn_about(q, apex, -close));
    }
    points.iter().all(|p| p.is_finite()).then_some(points)
}

/// Where two lines cross: the line through `a` along `da` and the line through `b` along `db`.
fn cross_point(a: Pt, da: Pt, b: Pt, db: Pt) -> Option<Pt> {
    let den = da.cross(db);
    if den.abs() < 1e-12 {
        return None;
    }
    let t = (b - a).cross(db) / den;
    let p = a + da * t;
    p.is_finite().then_some(p)
}

/// The cutting line of `outline`: each edge's allowance from `width` (in the outline's unit;
/// negative or not-a-number widths count as 0), each corner's style from `corner` (corner `i` is
/// where edge `i - 1` meets edge `i`, so it is called with the edge that starts there). The result
/// is a closed polyline, its first point not repeated at the end.
pub fn cutting_line(outline: &Outline, width: impl Fn(usize) -> f64, corner: impl Fn(usize) -> Corner) -> Result<Vec<Pt>, OutlineError> {
    let segs = outline.segments()?;
    let w = |s: &Seg| {
        let v = width(s.edge);
        if v.is_finite() && v > 0.0 { v } else { 0.0 }
    };
    let side_sign = {
        // +1 when a convex turn has a positive cross product of the directions.
        let s = segs[0];
        // The outward normal is the direction turned one way; which way depends on orientation.
        Pt::new(s.dir.y, -s.dir.x).dot(s.out).signum()
    };
    let n = segs.len();
    // Darts: the join at a dart's mouth (before its first leg) gives its cap, the joins at its
    // apex and its other mouth nothing.
    let mut caps: Vec<Option<Vec<Pt>>> = vec![None; n];
    let mut skip = vec![false; n];
    if n >= 5 {
        for k in 0..n {
            let (e1, leg1, leg2, e2) = (segs[(k + n - 1) % n], segs[k], segs[(k + 1) % n], segs[(k + 2) % n]);
            let taken =
                skip[k] || caps[k].is_some() || skip[(k + 1) % n] || skip[(k + 2) % n] || caps[(k + 1) % n].is_some() || caps[(k + 2) % n].is_some();
            let inward = leg1.dir.cross(leg2.dir) * side_sign < 0.0;
            let sharp = (-leg1.dir).dot(leg2.dir) > DART_DEGREES.to_radians().cos();
            if taken || !inward || !sharp {
                continue;
            }
            if let Some(cap) = dart_cap(&e1, &leg1, &leg2, &e2, w(&e1), w(&e2)) {
                caps[k] = Some(cap);
                skip[(k + 1) % n] = true;
                skip[(k + 2) % n] = true;
            }
        }
    }
    let mut out = Vec::with_capacity(n + 8);
    for j in 0..n {
        if skip[j] {
            continue;
        }
        if let Some(cap) = caps[j].take() {
            out.extend(cap);
            continue;
        }
        let a = segs[(j + n - 1) % n];
        let b = segs[j];
        let (wa, wb) = (w(&a), w(&b));
        let v = b.start;
        let end_a = v + a.out * wa;
        let start_b = v + b.out * wb;
        let is_corner = a.edge != b.edge;
        let turn = a.dir.cross(b.dir) * side_sign;
        let Some(m) = cross_point(end_a, a.dir, start_b, b.dir) else {
            // Straight on: one point, or a step where the allowance changes width.
            out.push(end_a);
            if (start_b - end_a).len() > EPS {
                out.push(start_b);
            }
            continue;
        };
        let widest = wa.max(wb);
        let reach = (m - v).len();
        let convex = turn > 0.0;
        if convex && is_corner && corner(b.edge) == Corner::Square && a.dir.dot(b.dir) < 0.0 {
            // Sharper than a right angle: cut square across.
            out.push(end_a + a.dir * wb);
            out.push(start_b - b.dir * wa);
        } else if widest > 0.0 && reach > widest * if is_corner { CORNER_MITRE_LIMIT } else { CURVE_MITRE_LIMIT } {
            out.push(end_a);
            out.push(start_b);
        } else {
            out.push(m);
        }
    }
    out.dedup_by(|a, b| (*a - *b).len() <= EPS);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A polygon whose every side is an edge of its own.
    fn polygon(points: &[(f64, f64)]) -> Outline {
        Outline { points: points.iter().map(|&(x, y)| Pt::new(x, y)).collect(), edge_of: (0..points.len()).collect() }
    }

    fn bounds(pts: &[Pt]) -> (f64, f64, f64, f64) {
        pts.iter().fold((f64::MAX, f64::MAX, f64::MIN, f64::MIN), |(a, b, c, d), p| (a.min(p.x), b.min(p.y), c.max(p.x), d.max(p.y)))
    }

    fn close(a: (f64, f64, f64, f64), b: (f64, f64, f64, f64)) -> bool {
        (a.0 - b.0).abs() < 1e-9 && (a.1 - b.1).abs() < 1e-9 && (a.2 - b.2).abs() < 1e-9 && (a.3 - b.3).abs() < 1e-9
    }

    #[test]
    fn a_square_grows_by_its_allowance_on_every_side_whichever_way_it_is_drawn() {
        let cw = polygon(&[(0.0, 0.0), (100.0, 0.0), (100.0, 100.0), (0.0, 100.0)]);
        let ccw = polygon(&[(0.0, 0.0), (0.0, 100.0), (100.0, 100.0), (100.0, 0.0)]);
        for o in [cw, ccw] {
            let cut = cutting_line(&o, |_| 10.0, |_| Corner::Mitre).unwrap();
            assert_eq!(cut.len(), 4, "{cut:?}");
            assert!(close(bounds(&cut), (-10.0, -10.0, 110.0, 110.0)), "{cut:?}");
        }
    }

    #[test]
    fn each_edge_takes_its_own_allowance_and_a_fold_takes_none() {
        // Edge 3 (the left side, x = 0) is on the fold: no allowance; the hem (edge 2, bottom) 40.
        let o = polygon(&[(0.0, 0.0), (100.0, 0.0), (100.0, 100.0), (0.0, 100.0)]);
        let w = |e: usize| match e {
            2 => 40.0,
            3 => 0.0,
            _ => 15.0,
        };
        let cut = cutting_line(&o, w, |_| Corner::Mitre).unwrap();
        assert!(close(bounds(&cut), (0.0, -15.0, 115.0, 140.0)), "{cut:?}");
    }

    #[test]
    fn a_square_corner_cuts_off_the_point_of_a_sharp_corner() {
        // An equilateral triangle: 60° corners, whose mitres reach twice the allowance out.
        let h = 100.0 * 3f64.sqrt();
        let o = polygon(&[(0.0, h), (100.0, 0.0), (200.0, h)]);
        let mitre = cutting_line(&o, |_| 10.0, |_| Corner::Mitre).unwrap();
        let square = cutting_line(&o, |_| 10.0, |c| if c == 1 { Corner::Square } else { Corner::Mitre }).unwrap();
        let top = |pts: &[Pt]| bounds(pts).1;
        assert!((top(&mitre) + 20.0).abs() < 1e-9, "the mitre's point: {}", top(&mitre));
        assert!(top(&square) > -15.0, "squared off: {}", top(&square));
        assert_eq!(square.len(), mitre.len() + 1);
        // A needle-sharp corner: even a mitre stops at four allowance widths, cut flat.
        let o = polygon(&[(0.0, 200.0), (20.0, 0.0), (40.0, 200.0)]);
        let needle = cutting_line(&o, |_| 10.0, |_| Corner::Mitre).unwrap();
        assert!(top(&needle) > -40.0, "{}", top(&needle));
        // At a right angle, square and mitre are the same.
        let sq = polygon(&[(0.0, 0.0), (100.0, 0.0), (100.0, 100.0), (0.0, 100.0)]);
        assert_eq!(cutting_line(&sq, |_| 10.0, |_| Corner::Square).unwrap(), cutting_line(&sq, |_| 10.0, |_| Corner::Mitre).unwrap());
    }

    #[test]
    fn an_inward_corner_is_trimmed_where_the_allowances_meet() {
        // An L: the corner at (50, 50) turns inward.
        let o = polygon(&[(0.0, 0.0), (100.0, 0.0), (100.0, 50.0), (50.0, 50.0), (50.0, 100.0), (0.0, 100.0)]);
        let cut = cutting_line(&o, |_| 10.0, |_| Corner::Mitre).unwrap();
        assert!(cut.iter().any(|p| (p.x - 60.0).abs() < 1e-9 && (p.y - 60.0).abs() < 1e-9), "{cut:?}");
        assert!(close(bounds(&cut), (-10.0, -10.0, 110.0, 110.0)));
    }

    #[test]
    fn a_curved_edge_keeps_its_distance_from_the_seam_line() {
        // A circle of radius 50 as one edge, flattened into 720 segments.
        let n = 720;
        let points: Vec<Pt> = (0..n)
            .map(|i| {
                let a = i as f64 / n as f64 * std::f64::consts::TAU;
                Pt::new(50.0 * a.cos(), 50.0 * a.sin())
            })
            .collect();
        let o = Outline { points, edge_of: vec![0; n] };
        let cut = cutting_line(&o, |_| 10.0, |_| Corner::Mitre).unwrap();
        for p in &cut {
            assert!((p.len() - 60.0).abs() < 0.01, "{}", p.len());
        }
    }

    #[test]
    fn a_dart_is_bridged_by_its_cap_not_cut_round() {
        // A 100 × 100 piece (y down) with a dart in its top edge: legs from (45, 0) and (55, 0)
        // to the point (50, 40), drawn either way round.
        let pts = [(0.0, 0.0), (45.0, 0.0), (50.0, 40.0), (55.0, 0.0), (100.0, 0.0), (100.0, 100.0), (0.0, 100.0)];
        let mut rev = pts;
        rev.reverse();
        for o in [polygon(&pts), polygon(&rev)] {
            let cut = cutting_line(&o, |_| 10.0, |_| Corner::Mitre).unwrap();
            // Nothing of the cutting line goes down into the dart: the allowance doesn't follow
            // the legs.
            assert!(cut.iter().all(|p| p.y <= 1e-9 || p.x < 0.0 || p.x > 100.0), "{cut:?}");
            // The cap: two points on the legs' lines above the mouth, mirror images in the
            // dart's middle line, and a peak on that line above them.
            let on_leg = |p: &Pt, a: Pt| ((*p - Pt::new(50.0, 40.0)).cross(a - Pt::new(50.0, 40.0))).abs() < 1e-6;
            let q1 = cut.iter().find(|p| p.y < 0.0 && on_leg(p, Pt::new(45.0, 0.0))).copied().unwrap();
            let q2 = cut.iter().find(|p| p.y < 0.0 && on_leg(p, Pt::new(55.0, 0.0))).copied().unwrap();
            assert!((q1.x + q2.x - 100.0).abs() < 1e-9 && (q1.y - q2.y).abs() < 1e-9, "{q1:?} {q2:?}");
            let peak = cut.iter().find(|p| (p.x - 50.0).abs() < 1e-9).copied().unwrap();
            assert!(peak.y < q1.y, "{peak:?} {q1:?}");
            // Folded, the cap lies on the allowance line: the peak mirrored in the first leg is
            // on the 10 mm line before the dart (y = -10) or on the folded line after it.
            let u = (Pt::new(45.0, 0.0) - Pt::new(50.0, 40.0)).unit().unwrap();
            let folded = reflect(peak, Pt::new(50.0, 40.0), u);
            assert!(folded.x < 50.0, "{folded:?}");
            // Elsewhere the allowance is the plain 10 mm.
            let (x0, _, x1, y1) = bounds(&cut);
            assert!((x0 + 10.0).abs() < 1e-9 && (x1 - 110.0).abs() < 1e-9 && (y1 - 110.0).abs() < 1e-9);
        }
    }

    #[test]
    fn the_dart_cap_folds_flat_onto_a_straight_waist() {
        // When the dart is closed the waist either side is one straight line (the sides tilt
        // to make it so): then the folded allowance is one line too, and the cap's peak mirrored
        // in the first leg lands on it.
        let apex = Pt::new(50.0, 60.0);
        let half = 5f64.atan2(60.0);
        // The waist after the dart turned by the dart's angle, so that closing the dart (which
        // turns it back) makes it one line with the waist before it.
        let b = Pt::new(55.0, 0.0);
        let far = turn_about(b + Pt::new(45.0, 0.0), b, 2.0 * half);
        let o = polygon(&[(0.0, 0.0), (45.0, 0.0), (apex.x, apex.y), (b.x, b.y), (far.x, far.y), (far.x, 100.0), (0.0, 100.0)]);
        let cut = cutting_line(&o, |_| 10.0, |_| Corner::Mitre).unwrap();
        let peak = cut.iter().find(|p| (p.x - 50.0).abs() < 1e-6 && p.y < 0.0).copied().unwrap();
        let u = (Pt::new(45.0, 0.0) - apex).unit().unwrap();
        assert!((reflect(peak, apex, u).y + 10.0).abs() < 1e-6, "{peak:?} {:?}", reflect(peak, apex, u));
    }

    #[test]
    fn on_a_sloping_waist_the_cap_still_folds_onto_the_allowance_line() {
        // A waist rising to the side, as a skirt's does: the cap follows the folded line, its
        // peak mirrored in the first leg on the allowance line where the dart lies folded.
        let apex = Pt::new(50.0, 80.0);
        let o = polygon(&[(0.0, 10.0), (45.0, 4.5), (apex.x, apex.y), (55.0, 4.0), (100.0, 0.0), (100.0, 100.0), (0.0, 100.0)]);
        let cut = cutting_line(&o, |_| 10.0, |_| Corner::Mitre).unwrap();
        let c = cut.iter().position(|p| (p.x - 50.0).abs() < 0.1 && p.y < 0.0).unwrap();
        let (q, q1, peak) = (cut[c - 2], cut[c - 1], cut[c]);
        let u = (Pt::new(45.0, 4.5) - apex).unit().unwrap();
        let folded = reflect(peak, apex, u);
        assert!((folded - q).cross(q1 - q).abs() / (q1 - q).len() < 1e-9, "{folded:?} not on {q:?}–{q1:?}");
        // The two points on the legs are as far from the apex.
        assert!(((q1 - apex).len() - (cut[c + 1] - apex).len()).abs() < 1e-9);
    }

    #[test]
    fn repeated_points_and_bad_input_do_not_break_it() {
        let o = polygon(&[(0.0, 0.0), (0.0, 0.0), (100.0, 0.0), (100.0, 100.0), (100.0, 100.0), (0.0, 100.0)]);
        let cut = cutting_line(&o, |_| 5.0, |_| Corner::Mitre).unwrap();
        assert!(close(bounds(&cut), (-5.0, -5.0, 105.0, 105.0)));
        assert_eq!(cutting_line(&polygon(&[(0.0, 0.0), (1.0, 1.0)]), |_| 5.0, |_| Corner::Mitre), Err(OutlineError::TooFewPoints));
        assert_eq!(cutting_line(&polygon(&[(0.0, 0.0), (1.0, 1.0), (2.0, 2.0)]), |_| 5.0, |_| Corner::Mitre), Err(OutlineError::NoArea));
        assert_eq!(cutting_line(&polygon(&[(0.0, 0.0), (f64::NAN, 1.0), (2.0, 0.0)]), |_| 5.0, |_| Corner::Mitre), Err(OutlineError::NotFinite));
        let bad = Outline { points: vec![Pt::new(0.0, 0.0); 3], edge_of: vec![0] };
        assert_eq!(cutting_line(&bad, |_| 5.0, |_| Corner::Mitre), Err(OutlineError::Malformed));
        // Negative and not-a-number widths count as none.
        let sq = polygon(&[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)]);
        let cut = cutting_line(&sq, |e| if e == 0 { f64::NAN } else { -3.0 }, |_| Corner::Mitre).unwrap();
        assert!(close(bounds(&cut), (0.0, 0.0, 10.0, 10.0)));
    }

    #[test]
    fn a_point_on_an_edge_is_found_by_length_with_its_outward_normal() {
        let o = polygon(&[(0.0, 0.0), (100.0, 0.0), (100.0, 100.0), (0.0, 100.0)]);
        let (p, dir, out) = o.point_on_edge(1, 0.25).unwrap();
        assert_eq!((p, dir, out), (Pt::new(100.0, 25.0), Pt::new(0.0, 1.0), Pt::new(1.0, 0.0)));
        assert!(o.point_on_edge(9, 0.5).is_none());
        // An edge of several segments that wraps round the start of the list.
        let o = Outline {
            points: vec![Pt::new(0.0, 0.0), Pt::new(50.0, 0.0), Pt::new(100.0, 0.0), Pt::new(100.0, 100.0), Pt::new(0.0, 100.0)],
            edge_of: vec![0, 0, 1, 2, 0],
        };
        assert_eq!(o.point_on_edge(0, 0.0).map(|r| r.0), Some(Pt::new(0.0, 100.0)));
        assert_eq!(o.point_on_edge(0, 0.75).map(|r| r.0), Some(Pt::new(50.0, 0.0)));
        assert_eq!(o.edges(), [0, 1, 2]);
    }
}
