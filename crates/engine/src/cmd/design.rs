//! Design tools (PF-5) on paths and pieces' seam lines: `design.rotateDart` pivots a dart to
//! another place on the outline, `design.split` cuts an outline in two (a princess seam through
//! the bust point), `design.length` measures along an outline (to true seams that sew together).
//! The geometry is [`pf_core::design`]. A block part or a drafted path that is changed this way
//! no longer follows the measurements (its points are no longer the block's).
use super::*;
use pf_core::design::{Knot, Side};
use pf_core::{Pt, pt_to_mm};
use serde_json::{Value, json};
use vectorcraft_doc::{Document, Node, NodeId, NodeKind};
use vectorcraft_geom::{Anchor, PathData, Point, SubPath};

pub fn specs() -> Vec<CommandSpec> {
    vec![
        cmd!(
            "design.rotateDart",
            "Rotate Dart",
            ["Pattern"],
            None,
            "{id? (a path or a piece; default: the selection), apex?: anchor (the dart's point; its neighbours are the legs), to?: [x, y] (document points: where on the outline the dart moves to) | toAnchor?: anchor, amount?: the share of the dart that moves (more than 0, up to 1; default 1), side?: \"auto\" (the side with fewer points swings) | \"before\" | \"after\"} pivot a dart about its point so it opens somewhere else on the outline; without apex and to, two selected points of one path: the sharper one is the dart's point → {id, apex (the new dart's point), anchors}",
            has_doc,
            rotate_dart
        ),
        cmd!(
            "design.split",
            "Split at Points",
            ["Pattern"],
            None,
            "{id? (a path; default: the selection), anchors?: [a, b] | [a] (default: the selected points)} cut a closed path in two between two of its points, each part closed by a straight seam; one point that another shares (a bust point two darts share) splits there with no seam added → {ids}",
            has_doc,
            split
        ),
        cmd!(
            query "design.length",
            "Seam Length",
            [],
            None,
            "{id? (a path or a piece; default: the selection), from: anchor, to: anchor} the length along the outline from one point forward to another, to true seams that sew together → {mm}",
            has_doc,
            length
        ),
    ]
}

fn err(cmd: &str, e: impl ToString) -> EngineError {
    bad(cmd, e.to_string())
}

fn to_pt(p: Point) -> Pt {
    Pt::new(p.x, p.y)
}

fn to_point(p: Pt) -> Point {
    Point::new(p.x, p.y)
}

/// The path's one closed outline as knots.
fn ring_of(doc: &Document, id: NodeId, cmd: &str) -> Result<Vec<Knot>> {
    let n = doc.node(id).ok_or(EngineError::NoNode(id))?;
    let NodeKind::Path { path, .. } = &n.kind else {
        return Err(bad(cmd, "select a path or a pattern piece"));
    };
    match path.subpaths.as_slice() {
        [sp] if sp.closed => Ok(sp.anchors.iter().map(|a| Knot { p: to_pt(a.p), h_in: to_pt(a.h_in), h_out: to_pt(a.h_out) }).collect()),
        _ => Err(bad(cmd, "the path must be one closed outline")),
    }
}

fn path_of(ring: &[Knot]) -> PathData {
    let anchors = ring.iter().map(|k| Anchor::with_handles(to_point(k.p), to_point(k.h_in), to_point(k.h_out))).collect();
    PathData::single(SubPath::new(anchors, true))
}

/// The node with a new outline; it no longer follows a block or point formulas.
fn reshaped(n: &Node, path: PathData) -> Node {
    let mut n = n.clone();
    if let NodeKind::Path { path: p, live, .. } = &mut n.kind {
        *p = path;
        *live = None;
    }
    n.extras_mut().block = None;
    n.extras_mut().draft = None;
    n
}

fn in_piece(doc: &Document, id: NodeId) -> bool {
    doc.parent_of(id).and_then(|g| doc.node(g)).is_some_and(|g| g.extras().pattern.as_deref().and_then(pf_core::PatternTag::piece).is_some())
}

fn anchor_param(p: &Value, key: &str, cmd: &str) -> Result<Option<usize>> {
    match p.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(v) => v
            .as_u64()
            .and_then(|a| usize::try_from(a).ok())
            .map(Some)
            .ok_or_else(|| bad(cmd, format!("{key}: a point's number along the path, from 0"))),
    }
}

/// The selected points of path `id`, in order along it.
fn selected_points(s: &Session, id: NodeId) -> Result<Vec<usize>> {
    let st = s.doc()?;
    Ok(st.selection.anchors.get(&id).map(|set| set.iter().filter(|(si, _)| *si == 0).map(|&(_, ai)| ai).collect()).unwrap_or_default())
}

fn rotate_dart(s: &mut Session, p: &Value) -> Result<Value> {
    const CMD: &str = "design.rotateDart";
    let id = super::measure::target_path(s, p, CMD)?;
    let mut ring = ring_of(&s.doc()?.doc, id, CMD)?;
    let amount = match p.get("amount") {
        None | Some(Value::Null) => 1.0,
        Some(v) => v.as_f64().ok_or_else(|| bad(CMD, "amount: a share of the dart, (0, 1]"))?,
    };
    let side = match str_param(p, "side") {
        None | Some("auto") => Side::Auto,
        Some("before") => Side::Before,
        Some("after") => Side::After,
        Some(o) => return Err(bad(CMD, format!("side: \"auto\", \"before\" or \"after\", not {o:?}"))),
    };
    let apex = anchor_param(p, "apex", CMD)?;
    let to_anchor = anchor_param(p, "toAnchor", CMD)?;
    let target = match p.get("to") {
        None | Some(Value::Null) => None,
        Some(v) => {
            let [x, y] = finite_numbers::<2>(v).ok_or_else(|| bad(CMD, "to: [x, y] in document points"))?;
            Some(Point::new(x, y))
        }
    };
    let (mut apex, at) = match (apex, to_anchor, target) {
        (Some(a), Some(t), None) => (a, Some(t)),
        (Some(a), None, Some(_)) => (a, None),
        (None, None, None) => {
            let [x, y] = selected_points(s, id)?[..] else {
                return Err(bad(CMD, "select two points of one path: the dart's point and where the dart moves to"));
            };
            let open = |i| pf_core::design::opening(&ring, i).unwrap_or(std::f64::consts::PI);
            if open(x) <= open(y) { (x, Some(y)) } else { (y, Some(x)) }
        }
        _ => return Err(bad(CMD, "give apex and one of to or toAnchor (or select the two points)")),
    };
    if apex >= ring.len() {
        return Err(bad(CMD, format!("the path has points 0 to {}", ring.len().saturating_sub(1))));
    }
    // A point on the outline: a new anchor there, unless one is already there.
    let at = match (at, target) {
        (Some(t), _) => t,
        (None, Some(q)) => {
            let path = path_of(&ring);
            let (_, seg, t, hit, dist) = path.nearest(q).ok_or_else(|| bad(CMD, "the path has no outline"))?;
            if dist > pf_core::mm_to_pt(5.0) {
                return Err(bad(CMD, "to: a point on the outline (within 5 mm of it)"));
            }
            match ring.iter().position(|k| (to_point(k.p) - hit).hypot() < 1e-6) {
                Some(i) => i,
                None => {
                    let mut sp = path.subpaths[0].clone();
                    let i = sp.insert_anchor(seg, t);
                    ring = sp.anchors.iter().map(|a| Knot { p: to_pt(a.p), h_in: to_pt(a.h_in), h_out: to_pt(a.h_out) }).collect();
                    if i <= apex {
                        apex += 1;
                    }
                    i
                }
            }
        }
        (None, None) => return Err(bad(CMD, "to or toAnchor: where the dart moves to")),
    };
    let (out, new_apex) = pf_core::design::rotate_dart(&ring, apex, at, amount, side).map_err(|e| err(CMD, e))?;
    let count = out.len();
    s.edit("Rotate Dart", |d, sel| {
        let n = d.node(id).ok_or(EngineError::NoNode(id))?;
        let new = reshaped(n, path_of(&out));
        if let Some(m) = d.node_mut(id) {
            *m = new;
        }
        sel.set([id]);
        Ok(())
    })?;
    Ok(json!({"id": id.0, "apex": new_apex, "anchors": count}))
}

fn split(s: &mut Session, p: &Value) -> Result<Value> {
    const CMD: &str = "design.split";
    let id = super::measure::target_path(s, p, CMD)?;
    let doc = &s.doc()?.doc;
    if in_piece(doc, id) {
        return Err(bad(CMD, "split the seam line on its own: release the piece first (Pattern › Release Pattern Piece)"));
    }
    let ring = ring_of(doc, id, CMD)?;
    let points: Vec<usize> = match p.get("anchors") {
        Some(Value::Array(a)) => a
            .iter()
            .map(|v| v.as_u64().and_then(|n| usize::try_from(n).ok()).ok_or_else(|| bad(CMD, "anchors: point numbers along the path")))
            .collect::<Result<_>>()?,
        Some(_) => return Err(bad(CMD, "anchors: [a, b] or [a]")),
        None => selected_points(s, id)?,
    };
    let (i, j) = match points[..] {
        [i, j] => (i, j),
        [i] => (
            i,
            pf_core::design::coincident(&ring, i)
                .ok_or_else(|| bad(CMD, "one point splits only where another point is at the same place (two darts' point)"))?,
        ),
        _ => return Err(bad(CMD, "select two points of one path (or one that two darts share)")),
    };
    let (a, b) = pf_core::design::split(&ring, i, j).map_err(|e| err(CMD, e))?;
    let ids = s.edit("Split at Points", |d, sel| {
        let n = d.node(id).ok_or(EngineError::NoNode(id))?.clone();
        let mut parts = Vec::new();
        for (k, ring) in [a, b].iter().enumerate() {
            let mut m = reshaped(&n, path_of(ring));
            m.id = d.alloc_id();
            m.name = n.name.as_ref().map(|nm| format!("{nm} {}", k + 1));
            parts.push(m);
        }
        let ids: Vec<NodeId> = parts.iter().map(|m| m.id).collect();
        d.insert_beside(vec![(id, parts)], true)?;
        sel.set(ids.iter().copied());
        Ok(ids)
    })?;
    Ok(json!({"ids": ids.iter().map(|i| i.0).collect::<Vec<_>>()}))
}

fn length(s: &mut Session, p: &Value) -> Result<Value> {
    const CMD: &str = "design.length";
    let id = super::measure::target_path(s, p, CMD)?;
    let ring = ring_of(&s.doc()?.doc, id, CMD)?;
    let (Some(from), Some(to)) = (anchor_param(p, "from", CMD)?, anchor_param(p, "to", CMD)?) else {
        return Err(bad(CMD, "from and to: point numbers along the path"));
    };
    let n = ring.len();
    if from >= n || to >= n {
        return Err(bad(CMD, format!("the path has points 0 to {}", n.saturating_sub(1))));
    }
    let path = path_of(&ring);
    let sp = &path.subpaths[0];
    let steps = (to + n - from) % n;
    let pts: f64 = (0..steps).map(|k| kurbo::ParamCurveArclen::arclen(&sp.segment((from + k) % n), 1e-6)).sum();
    Ok(json!({"mm": pt_to_mm(pts)}))
}
