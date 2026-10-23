//! Opening DXF-AAMA files (PF-7.8): a `.dxf` written to ASTM D6673 (its pieces as blocks with
//! "Piece Name:" text, [`pf_aama::read`]) opens as pattern pieces, not as plain drawing. Each piece's
//! sew line becomes its seam line (its cutting line when the file has none), and each edge's seam
//! allowance is how far the cutting line is from the edge's middle: the piece's allowance is the
//! middle value, and edges that differ keep their own. The notches, grainline, mirror line (the fold),
//! drill holes, internal lines, quantity and material come along. Millimetres, y up in the file;
//! points, y down in the document, the pieces where they were relative to each other.

use std::sync::Arc;

use pf_aama::{Mm, NotchShape, Style};
use pf_core::{Grainline, Notch, NotchKind, Piece, Pt, mm_to_pt};
use vectorcraft_color::{Color, Paint};
use vectorcraft_doc::{Appearance, Document, LayerColor, Node, Unit};
use vectorcraft_geom::{PathData, Point, SubPath};

use super::super::*;
use super::err;

/// Room round the pieces on the page, in mm.
const MARGIN_MM: f64 = 20.0;
/// How far an edge's allowance may be from the piece's (mm) and still be the piece's.
const SAME_MM: f64 = 0.05;
/// How near the mirror line a seam edge's ends must be (mm) to be the fold.
const ON_FOLD_MM: f64 = 0.5;

/// Whether `bytes` is a DXF-AAMA file: piece system text, and pieces that read.
pub(super) fn is_aama(bytes: &[u8]) -> bool {
    bytes.windows(11).any(|w| w.eq_ignore_ascii_case(b"Piece Name:")) && pf_aama::read(bytes).is_ok()
}

/// How far `q` is from the segment `a`–`b`, and how far along it (0 to 1) its nearest point is.
fn to_segment(q: Mm, a: Mm, b: Mm) -> (f64, f64) {
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let len2 = dx * dx + dy * dy;
    let t = if len2 > 0.0 { (((q[0] - a[0]) * dx + (q[1] - a[1]) * dy) / len2).clamp(0.0, 1.0) } else { 0.0 };
    let (px, py) = (a[0] + dx * t, a[1] + dy * t);
    (((q[0] - px).powi(2) + (q[1] - py).powi(2)).sqrt(), t)
}

/// The edge of the closed ring `ring` nearest `q`: (edge, along it 0 to 1, distance).
fn nearest_edge(ring: &[Mm], q: Mm) -> Option<(usize, f64, f64)> {
    (0..ring.len())
        .map(|i| {
            let (d, t) = to_segment(q, ring[i], ring[(i + 1) % ring.len()]);
            (i, t, d)
        })
        .min_by(|a, b| a.2.total_cmp(&b.2))
}

/// How far a point `q` is from the infinite line through `a` and `b`.
fn to_line(q: Mm, a: Mm, b: Mm) -> f64 {
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let len = (dx * dx + dy * dy).sqrt();
    if len == 0.0 {
        return ((q[0] - a[0]).powi(2) + (q[1] - a[1]).powi(2)).sqrt();
    }
    ((q[0] - a[0]) * dy - (q[1] - a[1]) * dx).abs() / len
}

/// The edges of the closed ring `ring` that are a dart's legs: two edges meeting in a sharp inward
/// corner, as [`pf_core::cutting_line`] finds darts.
fn dart_legs(ring: &[Mm]) -> Vec<usize> {
    let n = ring.len();
    let area: f64 = (0..n).map(|i| ring[i][0] * ring[(i + 1) % n][1] - ring[(i + 1) % n][0] * ring[i][1]).sum();
    let dir = |i: usize| {
        let (p, q) = (ring[i], ring[(i + 1) % n]);
        let len = (q[0] - p[0]).hypot(q[1] - p[1]);
        if len > 0.0 { [(q[0] - p[0]) / len, (q[1] - p[1]) / len] } else { [0.0, 0.0] }
    };
    let mut legs = Vec::new();
    if n < 5 {
        return legs;
    }
    let sharp = pf_core::allowance::DART_DEGREES.to_radians().cos();
    for i in 0..n {
        let (a, b) = (dir(i), dir((i + 1) % n));
        let turn = a[0] * b[1] - a[1] * b[0];
        if turn * area.signum() < 0.0 && -(a[0] * b[0] + a[1] * b[1]) > sharp {
            legs.extend([i, (i + 1) % n]);
        }
    }
    legs
}

/// The style's pieces as a document.
pub(super) fn import(bytes: &[u8]) -> Result<(Document, Vec<String>)> {
    let style: Style = pf_aama::read(bytes).map_err(|e| err(format!("can't open the DXF-AAMA file: {e}")))?;
    let (mut x0, mut y0, mut x1, mut y1) = (f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
    for q in style.pieces.iter().flat_map(|p| &p.boundary) {
        (x0, y0, x1, y1) = (x0.min(q[0]), y0.min(q[1]), x1.max(q[0]), y1.max(q[1]));
    }
    if !(x0.is_finite() && y0.is_finite() && x1.is_finite() && y1.is_finite()) || (x1 - x0).max(y1 - y0) > 100_000.0 {
        return Err(err("the DXF-AAMA file's pieces have no usable size"));
    }
    // File millimetres (y up) to document points (y down), the pieces a margin in from the page.
    let pt = |q: Mm| Pt::new(mm_to_pt(q[0] - x0 + MARGIN_MM), mm_to_pt(y1 - q[1] + MARGIN_MM));
    let point = |q: Mm| {
        let p = pt(q);
        Point::new(p.x, p.y)
    };
    let mut d = Document::new(mm_to_pt(x1 - x0 + 2.0 * MARGIN_MM), mm_to_pt(y1 - y0 + 2.0 * MARGIN_MM));
    d.units = Unit::Millimeters;
    d.layers.clear();
    let mut warnings = Vec::new();
    let mut groups = Vec::new();
    for ap in &style.pieces {
        let name = ap.name.trim().to_string();
        let sewn = ap.sew_line.len() >= 3;
        let seam = if sewn { &ap.sew_line } else { &ap.boundary };
        let edge = |i: usize| (seam[i], seam[(i + 1) % seam.len()]);
        // The fold: the longest seam edge lying on the mirror line.
        let fold = ap.mirror.and_then(|[a, b]| {
            let fold =
                (0..seam.len()).filter(|&i| to_line(edge(i).0, a, b) <= ON_FOLD_MM && to_line(edge(i).1, a, b) <= ON_FOLD_MM).max_by(|&i, &j| {
                    let len = |e: usize| {
                        let (p, q) = edge(e);
                        (q[0] - p[0]).hypot(q[1] - p[1])
                    };
                    len(i).total_cmp(&len(j))
                });
            if fold.is_none() {
                warnings.push(format!("{name}: its mirror line is not along its seam line, so it is not cut on the fold here"));
            }
            fold
        });
        // Each edge's allowance: how far the cutting line is from the edge's middle. The piece's
        // allowance is the middle value; edges further from it than a twentieth of a millimetre
        // keep their own. A dart's legs are left out: the cutting line bridges darts.
        let mut own = std::collections::BTreeMap::new();
        let mut allowance = 0.0;
        if sewn {
            let round = |v: f64| (v * 100.0).round() / 100.0;
            let legs = dart_legs(seam);
            let gaps: Vec<(usize, f64)> = (0..seam.len())
                .filter(|i| Some(*i) != fold && !legs.contains(i))
                .filter_map(|i| {
                    let (p, q) = edge(i);
                    nearest_edge(&ap.boundary, [(p[0] + q[0]) / 2.0, (p[1] + q[1]) / 2.0]).map(|e| (i, round(e.2)))
                })
                .collect();
            let mut sorted: Vec<f64> = gaps.iter().map(|g| g.1).collect();
            sorted.sort_by(f64::total_cmp);
            allowance = sorted.get(sorted.len() / 2).copied().unwrap_or_default();
            for (i, g) in gaps {
                if (g - allowance).abs() > SAME_MM {
                    own.insert(i, pf_core::piece::check_allowance(g).unwrap_or(pf_core::piece::MAX_ALLOWANCE_MM));
                }
            }
        }
        let allowance = pf_core::piece::check_allowance(allowance).unwrap_or(pf_core::piece::MAX_ALLOWANCE_MM);
        let (right, left) = ap.quantity;
        let mut piece = Piece {
            name: pf_core::piece::check_name(&name).unwrap_or_default(),
            cut: (right + left).clamp(1, pf_core::piece::MAX_CUT),
            mirror: left > 0,
            fabric: ap.material.trim().to_string(),
            allowance,
            edges: own,
            fold,
            ..Piece::default()
        };
        for n in &ap.notches {
            // On the cutting line in the file; on the nearest seam edge here.
            let Some((edge, at, _)) = nearest_edge(seam, n.at) else { continue };
            let kind = match n.shape {
                NotchShape::Plain if n.width > 0.0 => NotchKind::V,
                NotchShape::Plain => NotchKind::Slit,
                NotchShape::T => NotchKind::T,
                NotchShape::Castle => NotchKind::Castle,
                NotchShape::U => NotchKind::U,
            };
            if piece.add_notch(Notch { edge, at, kind }).is_err() {
                warnings.push(format!("{name}: only the first {} notches were read", pf_core::piece::MAX_NOTCHES));
                break;
            }
        }
        if let Some([a, b]) = ap.grain {
            piece.grainline = Some(Grainline { from: pt(a), to: pt(b) });
        }
        for dr in &ap.drills {
            let _ = piece.add_drill(pf_core::Drill { at: pt(dr.at), diameter: dr.diameter.clamp(0.0, pf_core::piece::MAX_DRILL_MM) });
        }
        let seam_node = Node::path(
            d.alloc_id(),
            PathData::single(SubPath::polyline(&seam.iter().map(|q| point(*q)).collect::<Vec<_>>(), true)),
            Appearance::default(),
        );
        let ink = Appearance::basic(Paint::None, Paint::solid(Color::BLACK), 0.75);
        let inside: Vec<Node> = ap
            .internal
            .iter()
            .filter(|l| l.len() >= 2)
            .map(|l| {
                Node::path(d.alloc_id(), PathData::single(SubPath::polyline(&l.iter().map(|q| point(*q)).collect::<Vec<_>>(), false)), ink.clone())
            })
            .collect();
        let gid = d.alloc_id();
        groups.push(super::super::piece::new_piece(gid, seam_node, piece, inside));
    }
    let ids: Vec<_> = groups.iter().map(|g| g.id).collect();
    let mut layer = Node::layer(d.alloc_id(), "Pattern pieces", LayerColor::Preset(1));
    if let Some(ch) = layer.children_mut() {
        *ch = groups.into_iter().map(Arc::new).collect();
    }
    d.layers.push(Arc::new(layer));
    d.pieces = true;
    for id in ids {
        super::super::piece::rebuild(&mut d, id)?;
    }
    if !style.sample_size.trim().is_empty() {
        warnings.push(format!("drafted in size {}; the file's grade rules are not read", style.sample_size.trim()));
    }
    Ok((d, warnings))
}
