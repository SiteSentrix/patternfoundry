//! The marks drawn on a piece: notches, the grainline arrow and drill holes.
//!
//! Every function returns open polylines in the outline's unit; the host strokes them.

use crate::Pt;
use crate::allowance::Outline;
use crate::piece::{Drill, Grainline, Notch, NotchKind};

/// How far a notch reaches in from the cutting line, in mm (a quarter inch is the usual snip).
pub const NOTCH_DEPTH_MM: f64 = 6.0;
/// How far apart the two slits of a double notch are, in mm.
pub const DOUBLE_NOTCH_GAP_MM: f64 = 4.0;
/// How wide a T notch's bar and a castle or U notch's cut are, in mm.
pub const NOTCH_WIDTH_MM: f64 = 3.0;
/// How long a grainline's arrowheads are, in mm.
pub const ARROW_MM: f64 = 6.0;
/// How far a drill hole's cross reaches each way from its centre, in mm.
pub const DRILL_CROSS_MM: f64 = 3.0;

/// The marks of `notch`: one polyline per slit (or one V). `allowance` is the notch's edge's seam
/// allowance and `mm` one millimetre, both in the outline's unit. A notch is cut in from the
/// cutting line toward the seam line, [`NOTCH_DEPTH_MM`] deep but never past the seam line when
/// there is an allowance; on an edge without allowance it is cut in from the seam line. The shapes
/// are those of ASTM D6673 as DH Patterns and Fit's overview describes them: a T notch is a slit
/// with a bar [`NOTCH_WIDTH_MM`] wide at its depth, a castle notch a cut of even width with a square
/// end, a U notch the same with a semicircle reaching the depth.
pub fn notch(outline: &Outline, n: &Notch, allowance: f64, mm: f64) -> Vec<Vec<Pt>> {
    let Some((p, dir, out)) = outline.point_on_edge(n.edge, n.at) else {
        return vec![];
    };
    let allowance = if allowance.is_finite() && allowance > 0.0 { allowance } else { 0.0 };
    let depth = if allowance > 0.0 { (NOTCH_DEPTH_MM * mm).min(allowance) } else { NOTCH_DEPTH_MM * mm };
    let outer = p + out * allowance;
    let inner = outer - out * depth;
    let slit = |shift: f64| vec![outer + dir * shift, inner + dir * shift];
    match n.kind {
        NotchKind::Slit => vec![slit(0.0)],
        NotchKind::Double => {
            let h = DOUBLE_NOTCH_GAP_MM * mm / 2.0;
            vec![slit(-h), slit(h)]
        }
        NotchKind::V => {
            let h = depth / 2.0;
            vec![vec![outer - dir * h, inner, outer + dir * h]]
        }
        NotchKind::T => {
            let h = NOTCH_WIDTH_MM * mm / 2.0;
            vec![vec![outer, inner], vec![inner - dir * h, inner + dir * h]]
        }
        NotchKind::Castle => {
            let h = NOTCH_WIDTH_MM * mm / 2.0;
            vec![vec![outer - dir * h, inner - dir * h, inner + dir * h, outer + dir * h]]
        }
        NotchKind::U => {
            let h = (NOTCH_WIDTH_MM * mm / 2.0).min(depth);
            // The round end's centre, a half-width back from the depth.
            let c = inner + out * h;
            let mut line = vec![outer - dir * h];
            const STEPS: usize = 8;
            for i in 0..=STEPS {
                let (s, k) = (i as f64 * std::f64::consts::PI / STEPS as f64).sin_cos();
                line.push(c - dir * (h * k) - out * (h * s));
            }
            line.push(outer + dir * h);
            vec![line]
        }
    }
}

/// The grainline drawn as polylines: the line, and an arrowhead at each end (the grain runs both
/// ways). `mm` is one millimetre in the outline's unit. Nothing for a grainline of no length.
pub fn grainline(g: &Grainline, mm: f64) -> Vec<Vec<Pt>> {
    let Some(d) = (g.to - g.from).unit() else {
        return vec![];
    };
    let n = Pt::new(-d.y, d.x);
    let a = ARROW_MM * mm;
    let head = |tip: Pt, back: Pt| vec![tip + back * a + n * (a / 2.0), tip, tip + back * a - n * (a / 2.0)];
    vec![vec![g.from, g.to], head(g.to, -d), head(g.from, d)]
}

/// A grainline for a piece with no grainline yet: vertical (the lengthwise grain runs down the
/// page), through the middle of the outline's bounds, two thirds of its height long.
pub fn default_grainline(outline: &Outline) -> Option<Grainline> {
    let first = outline.points.first()?;
    let (mut x0, mut y0, mut x1, mut y1) = (first.x, first.y, first.x, first.y);
    for p in &outline.points {
        if !p.is_finite() {
            return None;
        }
        x0 = x0.min(p.x);
        y0 = y0.min(p.y);
        x1 = x1.max(p.x);
        y1 = y1.max(p.y);
    }
    let (cx, cy, h) = ((x0 + x1) / 2.0, (y0 + y1) / 2.0, y1 - y0);
    (h > 0.0).then(|| Grainline { from: Pt::new(cx, cy - h / 3.0), to: Pt::new(cx, cy + h / 3.0) })
}

/// A drill hole drawn as polylines: a cross [`DRILL_CROSS_MM`] each way (as cutting rooms show a
/// drill point), and the punch's circle when it has a diameter. `mm` is one millimetre in the
/// outline's unit.
pub fn drill(d: &Drill, mm: f64) -> Vec<Vec<Pt>> {
    let c = d.at;
    let a = DRILL_CROSS_MM * mm;
    let mut out = vec![vec![c + Pt::new(-a, -a), c + Pt::new(a, a)], vec![c + Pt::new(-a, a), c + Pt::new(a, -a)]];
    if d.diameter > 0.0 {
        let r = d.diameter * mm / 2.0;
        const SIDES: usize = 32;
        out.push(
            (0..=SIDES)
                .map(|i| {
                    let (s, k) = (i as f64 * std::f64::consts::TAU / SIDES as f64).sin_cos();
                    c + Pt::new(k * r, s * r)
                })
                .collect(),
        );
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square() -> Outline {
        let points = [(0.0, 0.0), (100.0, 0.0), (100.0, 100.0), (0.0, 100.0)].iter().map(|&(x, y)| Pt::new(x, y)).collect();
        Outline { points, edge_of: vec![0, 1, 2, 3] }
    }

    #[test]
    fn a_notch_is_snipped_in_from_the_cutting_line() {
        // The right side, half way down, 10 units of allowance, 1 unit per mm.
        let n = Notch { edge: 1, at: 0.5, kind: NotchKind::Slit };
        assert_eq!(notch(&square(), &n, 10.0, 1.0), [vec![Pt::new(110.0, 50.0), Pt::new(104.0, 50.0)]]);
        // A narrow allowance: the snip stops at the seam line.
        assert_eq!(notch(&square(), &n, 3.0, 1.0), [vec![Pt::new(103.0, 50.0), Pt::new(100.0, 50.0)]]);
        // No allowance (a fold): cut in from the seam line.
        assert_eq!(notch(&square(), &n, 0.0, 1.0), [vec![Pt::new(100.0, 50.0), Pt::new(94.0, 50.0)]]);
    }

    #[test]
    fn double_and_v_notches() {
        let d = notch(&square(), &Notch { edge: 0, at: 0.5, kind: NotchKind::Double }, 10.0, 1.0);
        assert_eq!(d, [vec![Pt::new(48.0, -10.0), Pt::new(48.0, -4.0)], vec![Pt::new(52.0, -10.0), Pt::new(52.0, -4.0)]]);
        let v = notch(&square(), &Notch { edge: 0, at: 0.5, kind: NotchKind::V }, 10.0, 1.0);
        assert_eq!(v, [vec![Pt::new(47.0, -10.0), Pt::new(50.0, -4.0), Pt::new(53.0, -10.0)]]);
        assert!(notch(&square(), &Notch { edge: 7, at: 0.5, kind: NotchKind::V }, 10.0, 1.0).is_empty());
        // The ASTM shapes: a T's bar at the depth, a castle's square end, a U's round end.
        let t = notch(&square(), &Notch { edge: 0, at: 0.5, kind: NotchKind::T }, 10.0, 1.0);
        assert_eq!(t, [vec![Pt::new(50.0, -10.0), Pt::new(50.0, -4.0)], vec![Pt::new(48.5, -4.0), Pt::new(51.5, -4.0)]]);
        let c = notch(&square(), &Notch { edge: 0, at: 0.5, kind: NotchKind::Castle }, 10.0, 1.0);
        assert_eq!(c, [vec![Pt::new(48.5, -10.0), Pt::new(48.5, -4.0), Pt::new(51.5, -4.0), Pt::new(51.5, -10.0)]]);
        let u = &notch(&square(), &Notch { edge: 0, at: 0.5, kind: NotchKind::U }, 10.0, 1.0)[0];
        let deepest = u.iter().map(|p| p.y).fold(f64::NEG_INFINITY, f64::max);
        assert!((deepest + 4.0).abs() < 1e-9 && (u[0].x - 48.5).abs() < 1e-9 && (u[u.len() - 1].x - 51.5).abs() < 1e-9, "{u:?}");
    }

    #[test]
    fn the_grainline_has_a_head_at_each_end_and_defaults_to_vertical() {
        let g = default_grainline(&square()).unwrap();
        assert!((g.from.y - 100.0 / 6.0).abs() < 1e-9 && (g.to.y - 500.0 / 6.0).abs() < 1e-9 && g.from.x == 50.0);
        let lines = grainline(&g, 1.0);
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[1][1], g.to);
        assert_eq!(lines[2][1], g.from);
        assert!(grainline(&Grainline { from: Pt::new(1.0, 1.0), to: Pt::new(1.0, 1.0) }, 1.0).is_empty());
    }

    #[test]
    fn a_drill_hole_is_a_cross_and_its_punch_circle() {
        let mark = drill(&Drill { at: Pt::new(10.0, 20.0), diameter: 0.0 }, 1.0);
        assert_eq!(mark, vec![vec![Pt::new(7.0, 17.0), Pt::new(13.0, 23.0)], vec![Pt::new(7.0, 23.0), Pt::new(13.0, 17.0)]]);
        let punched = drill(&Drill { at: Pt::new(0.0, 0.0), diameter: 4.0 }, 2.0);
        assert_eq!(punched.len(), 3);
        assert!(punched[2].iter().all(|p| (p.len() - 4.0).abs() < 1e-9), "radius 2 mm at 2 units a mm");
    }
}
