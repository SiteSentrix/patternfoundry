//! Marker making (PF-9): every piece to cut laid on a length of fabric of a given width, as
//! closely as it goes, so the marker (the cutting layout) is as short as it can be.
//!
//! The fabric runs along x (its length) from x = 0, its width along y from 0 to `width`. Each
//! [`Part`] comes already turned so its grainline runs along x, as the grain must run along the
//! fabric's length; a part may also be laid turned half round (180°) unless the fabric has a nap
//! or a one-way print. Parts are laid largest first, each where its right end comes furthest
//! left (then nearest the top edge): a *bottom-left fill*, the classic greedy heuristic for
//! irregular nesting (Hopper & Turton, "An empirical investigation of meta-heuristic and
//! heuristic algorithms for a 2D packing problem", EJOR 128 (2001)). Overlap is tested on a grid
//! of `step` cells: each part covers every cell its outline, grown by half the gap, touches, so
//! parts laid never overlap and are at least `gap` apart; a finer step lays them closer, at more
//! cost.
//!
//! Every length is in the parts' own unit.

use crate::Pt;

/// Most parts one marker lays.
pub const MAX_PARTS: usize = 5_000;
/// Most grid cells a marker uses (fabric rows × columns): a bound on memory and time.
pub const MAX_CELLS: usize = 40_000_000;
/// Most points one part's outline has.
pub const MAX_POINTS: usize = 100_000;

/// A part to lay: its outline (a polygon, grainline along x), and whether it may be turned 180°.
#[derive(Clone, Debug, PartialEq)]
pub struct Part {
    pub points: Vec<Pt>,
    pub turn: bool,
}

/// Where a part went: its points, turned half round about the origin when `turned`, moved by
/// `offset`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Placed {
    pub offset: Pt,
    pub turned: bool,
}

impl Placed {
    /// Where point `p` of the part lies on the fabric.
    pub fn apply(&self, p: Pt) -> Pt {
        let q = if self.turned { Pt::new(-p.x, -p.y) } else { p };
        q + self.offset
    }
}

/// A laid marker.
#[derive(Clone, Debug, PartialEq)]
pub struct Layout {
    /// Each part's place, in the order given; `None` for a part wider than the fabric.
    pub placed: Vec<Option<Placed>>,
    /// The marker's length: from x = 0 to the furthest point laid.
    pub length: f64,
}

/// Why a marker can't be laid.
#[derive(Clone, Debug, PartialEq)]
pub enum MarkerError {
    /// A width not more than 0, or not a number.
    Width,
    /// A gap below 0, or not a number.
    Gap,
    /// A grid step not more than 0, or not a number.
    Step,
    /// More than [`MAX_PARTS`] parts, or a part with too many points.
    TooMany,
    /// The grid would take more than [`MAX_CELLS`] cells.
    TooLarge,
}

impl std::fmt::Display for MarkerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            MarkerError::Width => "the fabric's width must be more than 0",
            MarkerError::Gap => "the gap must be 0 or more",
            MarkerError::Step => "the precision must be more than 0",
            MarkerError::TooMany => "too many pieces to lay at once",
            MarkerError::TooLarge => "the marker is too large at this precision: give a coarser one",
        })
    }
}

impl std::error::Error for MarkerError {}

/// The area inside polygon `points` (positive whichever way it runs).
pub fn area(points: &[Pt]) -> f64 {
    let n = points.len();
    (0..n).map(|i| points[i].cross(points[(i + 1) % n])).sum::<f64>().abs() / 2.0
}

/// A part's cells: per grid row, from its top, half-open runs of columns.
struct Cells {
    cols: usize,
    rows: Vec<Vec<(usize, usize)>>,
}

/// The x-ranges where polygon `pts` meets the band `y0..=y1`: each edge's x-range in the band,
/// and the inside of the polygon along the band's middle line (a point of the polygon in the
/// band either has the outline above or below it in the band, or the whole height of the band
/// inside the polygon there, so this is all of it).
fn band(pts: &[Pt], y0: f64, y1: f64) -> Vec<(f64, f64)> {
    let n = pts.len();
    let mut out = Vec::new();
    for i in 0..n {
        let (p, q) = (pts[i], pts[(i + 1) % n]);
        let (lo, hi) = (p.y.min(q.y), p.y.max(q.y));
        if hi < y0 || lo > y1 {
            continue;
        }
        let x_at = |y: f64| if (q.y - p.y).abs() < 1e-12 { p.x } else { p.x + (q.x - p.x) * (y - p.y) / (q.y - p.y) };
        let (a, b) = (x_at(lo.max(y0)), x_at(hi.min(y1)));
        let (a, b) = if (q.y - p.y).abs() < 1e-12 { (p.x, q.x) } else { (a, b) };
        out.push((a.min(b), a.max(b)));
    }
    let y = (y0 + y1) / 2.0;
    let mut xs: Vec<f64> = (0..n)
        .filter_map(|i| {
            let (p, q) = (pts[i], pts[(i + 1) % n]);
            ((p.y <= y) != (q.y <= y)).then(|| p.x + (q.x - p.x) * (y - p.y) / (q.y - p.y))
        })
        .collect();
    xs.sort_by(f64::total_cmp);
    out.extend(xs.chunks_exact(2).map(|c| (c[0], c[1])));
    out
}

/// The cells polygon `pts` (with its smallest x and y at `pad`) covers, grown by `pad`.
fn cells(pts: &[Pt], pad: f64, step: f64) -> Cells {
    let (w, h) = pts.iter().fold((0.0f64, 0.0f64), |(w, h), p| (w.max(p.x), h.max(p.y)));
    let cols = ((w + pad) / step).ceil().max(1.0) as usize;
    let n_rows = ((h + pad) / step).ceil().max(1.0) as usize;
    let mut rows = Vec::with_capacity(n_rows);
    for j in 0..n_rows {
        let (y0, y1) = (j as f64 * step - pad, (j + 1) as f64 * step + pad);
        let mut runs: Vec<(usize, usize)> = band(pts, y0, y1)
            .into_iter()
            .map(|(a, b)| (((a - pad) / step).floor().max(0.0) as usize, (((b + pad) / step).ceil() as usize).clamp(1, cols)))
            .filter(|(a, b)| a < b)
            .collect();
        runs.sort_unstable();
        let mut merged: Vec<(usize, usize)> = Vec::new();
        for (a, b) in runs {
            match merged.last_mut() {
                Some(last) if a <= last.1 => last.1 = last.1.max(b),
                _ => merged.push((a, b)),
            }
        }
        rows.push(merged);
    }
    Cells { cols, rows }
}

/// The fabric's grid: a bit per cell, set where a part lies.
struct Grid {
    words: usize,
    bits: Vec<u64>,
}

impl Grid {
    fn mask(from: usize, to: usize) -> impl Iterator<Item = (usize, u64)> {
        let (first, last) = (from / 64, (to - 1) / 64);
        (first..=last).map(move |w| {
            let lo = if w == first { from % 64 } else { 0 };
            let hi = if w == last { (to - 1) % 64 + 1 } else { 64 };
            let m = if hi - lo == 64 { u64::MAX } else { ((1u64 << (hi - lo)) - 1) << lo };
            (w, m)
        })
    }
    fn free(&self, row: usize, from: usize, to: usize) -> bool {
        let base = row * self.words;
        Self::mask(from, to).all(|(w, m)| self.bits[base + w] & m == 0)
    }
    fn fill(&mut self, row: usize, from: usize, to: usize) {
        let base = row * self.words;
        for (w, m) in Self::mask(from, to) {
            self.bits[base + w] |= m;
        }
    }
    fn fits(&self, c: &Cells, cx: usize, cy: usize) -> bool {
        c.rows.iter().enumerate().all(|(r, runs)| runs.iter().all(|&(a, b)| self.free(cy + r, cx + a, cx + b)))
    }
    fn put(&mut self, c: &Cells, cx: usize, cy: usize) {
        for (r, runs) in c.rows.iter().enumerate() {
            for &(a, b) in runs {
                self.fill(cy + r, cx + a, cx + b);
            }
        }
    }
}

/// One way a part can lie: its cells, and how its points map into them.
struct Way {
    cells: Cells,
    turned: bool,
    /// What moves the (turned) points so their smallest x and y are at the pad.
    shift: Pt,
}

fn way(points: &[Pt], turned: bool, pad: f64, step: f64) -> Way {
    let pts: Vec<Pt> = points.iter().map(|p| if turned { Pt::new(-p.x, -p.y) } else { *p }).collect();
    let (mx, my) = pts.iter().fold((f64::INFINITY, f64::INFINITY), |(x, y), p| (x.min(p.x), y.min(p.y)));
    let shift = Pt::new(pad - mx, pad - my);
    let moved: Vec<Pt> = pts.iter().map(|p| *p + shift).collect();
    Way { cells: cells(&moved, pad, step), turned, shift }
}

/// Lay `parts` across fabric `width` wide, `gap` apart, testing overlap on a grid of `step`.
pub fn lay(parts: &[Part], width: f64, gap: f64, step: f64) -> Result<Layout, MarkerError> {
    if !(width.is_finite() && width > 0.0) {
        return Err(MarkerError::Width);
    }
    if !(gap.is_finite() && gap >= 0.0) {
        return Err(MarkerError::Gap);
    }
    if !(step.is_finite() && step > 0.0) {
        return Err(MarkerError::Step);
    }
    if parts.len() > MAX_PARTS || parts.iter().any(|p| p.points.len() > MAX_POINTS) {
        return Err(MarkerError::TooMany);
    }
    let pad = gap / 2.0;
    // The grid starts half a gap before the fabric's top edge and its start, and ends half a gap
    // past its bottom edge: the gap is kept between parts, not from the edges.
    let fabric_rows = ((width + gap) / step).floor() as usize;
    let mut placed = vec![None; parts.len()];
    // Outlines that are no polygon, or not finite, are not laid.
    let usable = |p: &Part| p.points.len() >= 3 && p.points.iter().all(|q| q.is_finite());
    let ways: Vec<Vec<Way>> = parts
        .iter()
        .map(|p| {
            if !usable(p) {
                return Vec::new();
            }
            let mut w = vec![way(&p.points, false, pad, step)];
            if p.turn {
                w.push(way(&p.points, true, pad, step));
            }
            w
        })
        .collect();
    // The grid is as long as every part laid end to end.
    let total: usize = ways.iter().filter_map(|w| w.first()).map(|w| w.cells.cols).sum::<usize>() + 1;
    if fabric_rows.saturating_mul(total) > MAX_CELLS {
        return Err(MarkerError::TooLarge);
    }
    let words = total.div_ceil(64);
    let mut grid = Grid { words, bits: vec![0; words * fabric_rows] };
    let mut order: Vec<usize> = (0..parts.len()).collect();
    order.sort_by(|&a, &b| area(&parts[b].points).total_cmp(&area(&parts[a].points)).then(a.cmp(&b)));
    let mut length = 0.0f64;
    for i in order {
        // The way whose right end comes furthest left, then nearest the top.
        let mut best: Option<(usize, usize, usize)> = None;
        for (k, w) in ways[i].iter().enumerate() {
            let c = &w.cells;
            if c.rows.len() > fabric_rows || c.cols > total {
                continue;
            }
            let found = (0..=total - c.cols).find_map(|cx| (0..=fabric_rows - c.rows.len()).find(|&cy| grid.fits(c, cx, cy)).map(|cy| (cx, cy)));
            if let Some((cx, cy)) = found {
                let key = (cx + c.cols, cy);
                if best.is_none_or(|(bk, bx, by)| key < (bx + ways[i][bk].cells.cols, by)) {
                    best = Some((k, cx, cy));
                }
            }
        }
        let Some((k, cx, cy)) = best else { continue };
        let w = &ways[i][k];
        grid.put(&w.cells, cx, cy);
        let at = Placed { offset: w.shift + Pt::new(cx as f64 * step - pad, cy as f64 * step - pad), turned: w.turned };
        length = parts[i].points.iter().map(|p| at.apply(*p).x).fold(length, f64::max);
        placed[i] = Some(at);
    }
    Ok(Layout { placed, length })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(w: f64, h: f64) -> Vec<Pt> {
        vec![Pt::new(0.0, 0.0), Pt::new(w, 0.0), Pt::new(w, h), Pt::new(0.0, h)]
    }

    /// Whether two convex or concave polygons' insides overlap, by sampling a fine grid.
    fn overlap(a: &[Pt], b: &[Pt]) -> bool {
        let inside = |pts: &[Pt], q: Pt| {
            let n = pts.len();
            let mut c = false;
            for i in 0..n {
                let (p, r) = (pts[i], pts[(i + 1) % n]);
                if (p.y > q.y) != (r.y > q.y) && q.x < p.x + (r.x - p.x) * (q.y - p.y) / (r.y - p.y) {
                    c = !c;
                }
            }
            c
        };
        let (lo, hi) = a.iter().fold((Pt::new(f64::INFINITY, f64::INFINITY), Pt::new(f64::NEG_INFINITY, f64::NEG_INFINITY)), |(l, h), p| {
            (Pt::new(l.x.min(p.x), l.y.min(p.y)), Pt::new(h.x.max(p.x), h.y.max(p.y)))
        });
        let n = 60;
        (0..=n).any(|i| {
            (0..=n).any(|j| {
                let q = Pt::new(lo.x + (hi.x - lo.x) * i as f64 / n as f64, lo.y + (hi.y - lo.y) * j as f64 / n as f64);
                inside(a, q) && inside(b, q)
            })
        })
    }

    #[test]
    fn rectangles_fill_the_width_then_the_length() {
        // Four 500 × 700 pieces on 1400 wide fabric: two across, two along, 1000 long.
        let parts: Vec<Part> = (0..4).map(|_| Part { points: rect(500.0, 700.0), turn: true }).collect();
        let l = lay(&parts, 1400.0, 0.0, 10.0).unwrap();
        assert!(l.placed.iter().all(Option::is_some));
        assert!((l.length - 1000.0).abs() < 1e-9, "{}", l.length);
        // With a 10 gap, the second pair starts 10 further on.
        let l = lay(&parts, 1410.0, 10.0, 5.0).unwrap();
        assert!((l.length - 1010.0).abs() < 5.0 + 1e-9, "{}", l.length);
    }

    #[test]
    fn laid_parts_never_overlap_and_stay_on_the_fabric() {
        // An L, a triangle and rectangles: concave shapes nest into each other.
        let l_shape =
            vec![Pt::new(0.0, 0.0), Pt::new(400.0, 0.0), Pt::new(400.0, 100.0), Pt::new(100.0, 100.0), Pt::new(100.0, 500.0), Pt::new(0.0, 500.0)];
        let tri = vec![Pt::new(0.0, 0.0), Pt::new(300.0, 50.0), Pt::new(20.0, 400.0)];
        let mut parts = vec![Part { points: l_shape.clone(), turn: true }, Part { points: l_shape, turn: true }];
        parts.extend((0..3).map(|_| Part { points: tri.clone(), turn: true }));
        parts.extend((0..3).map(|_| Part { points: rect(250.0, 120.0), turn: false }));
        let width = 900.0;
        let l = lay(&parts, width, 4.0, 5.0).unwrap();
        let laid: Vec<Vec<Pt>> = parts.iter().zip(&l.placed).map(|(p, at)| p.points.iter().map(|q| at.unwrap().apply(*q)).collect()).collect();
        for pts in &laid {
            assert!(pts.iter().all(|p| p.y >= -1e-9 && p.y <= width + 1e-9 && p.x >= -1e-9 && p.x <= l.length + 1e-9));
        }
        for i in 0..laid.len() {
            for j in i + 1..laid.len() {
                assert!(!overlap(&laid[i], &laid[j]), "parts {i} and {j} overlap");
            }
        }
        // Closer than laying them in a row.
        assert!(l.length < 1500.0, "{}", l.length);
    }

    #[test]
    fn a_one_way_part_is_never_turned_and_a_too_wide_one_is_left_out() {
        let tri = vec![Pt::new(0.0, 0.0), Pt::new(300.0, 0.0), Pt::new(0.0, 300.0)];
        let parts =
            vec![Part { points: tri.clone(), turn: false }, Part { points: tri, turn: false }, Part { points: rect(100.0, 2000.0), turn: true }];
        let l = lay(&parts, 300.0, 0.0, 5.0).unwrap();
        assert!(l.placed[..2].iter().all(|p| p.is_some_and(|p| !p.turned)));
        assert_eq!(l.placed[2], None);
        // Turning lets two triangles make a square.
        let parts: Vec<Part> =
            (0..2).map(|_| Part { points: vec![Pt::new(0.0, 0.0), Pt::new(300.0, 0.0), Pt::new(0.0, 300.0)], turn: true }).collect();
        let l = lay(&parts, 300.0, 0.0, 5.0).unwrap();
        assert!(l.length <= 310.0, "{}", l.length);
        assert_eq!(lay(&parts, 0.0, 0.0, 5.0), Err(MarkerError::Width));
        assert_eq!(lay(&parts, 300.0, -1.0, 5.0), Err(MarkerError::Gap));
        assert_eq!(lay(&parts, 300.0, 0.0, 0.0), Err(MarkerError::Step));
    }
}
