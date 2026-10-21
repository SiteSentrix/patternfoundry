//! The cutting line: a piece's seam line pushed outward by each edge's seam allowance.
//!
//! The seam line arrives flattened, as an [`Outline`]: a closed polyline whose segments each know
//! the edge of the piece they belong to. Each segment's allowance line is the segment moved
//! outward by its edge's width; neighbouring allowance lines are joined where they cross. Where two
//! edges meet (a corner) the piece's [`Corner`] style decides the join; inside a curved edge the
//! joins are mitres, cut off (bevelled) where a mitre would reach far out.
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
    let mut out = Vec::with_capacity(n + 8);
    for j in 0..n {
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
