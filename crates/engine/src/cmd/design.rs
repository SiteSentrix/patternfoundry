//! Design tools (PF-5) on paths and pieces' seam lines: `design.rotateDart` pivots a dart to
//! another place on the outline, `design.split` cuts an outline in two (a princess seam through
//! the bust point), `design.spread` slashes and spreads an outline (flare, fullness),
//! `design.pocket` makes a patch pocket and its placement mark, `design.collar` drafts a flat collar from the bodice's necklines, `design.gather` marks an edge to gather, `design.pleat` adds a knife pleat with its marks, `design.facing` makes a facing for an edge, `design.length` measures along an outline and `design.matchLength` trues a seam to a length or
//! to another seam (seams that sew together).
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
            "design.spread",
            "Slash and Spread",
            ["Pattern"],
            None,
            "{id? (a path or a piece; default: the selection), from: anchor | [x, y] (document points on the outline: where it opens), to: anchor | [x, y] (the slash's other end), amount: mm (how far it opens at from), mode?: \"hinge\" (default: the gap closes to nothing at to; flare) | \"parallel\" (both ends open; even fullness or a pleat)} slash the outline straight from one point to another and spread it open; the points from `from` forward to `to` stay → {id, anchors}",
            has_doc,
            spread
        ),
        cmd!(
            "design.facing",
            "Make Facing",
            ["Pattern"],
            None,
            "{id? (a path or a piece; default: the selection), from: anchor | [x, y] (document points on the outline), to: anchor | [x, y] (the edge to face runs from one forward to the other), width?: mm (50)} a facing for an edge (without from and to: between two selected points, the shorter way round) (a neckline, an armhole): a new path whose outer edge is that run of the outline and whose inner edge is the run moved the width inward, its ends along the seams either side; put above the path or its piece → {id, anchors}",
            has_doc,
            facing
        ),
        cmd!(
            "design.pleat",
            "Add Pleat",
            ["Pattern"],
            None,
            "{id? (a path or a piece; default: the selection), from: anchor | [x, y] (document points on the outline), to: anchor | [x, y] (the pleat's line runs from one to the other), depth: mm (how far it folds under), kind?: \"knife\" (default) | \"inverted\"} a pleat: for a knife pleat the outline spread parallel by twice the depth, folding toward the side that stays (from forward to to); for an inverted pleat four times, both edges folded to the middle. Its fold lines (solid, with arrows to the placements) and placement lines (dashed) added in the piece or above the path → {id, anchors, fold, placement}",
            has_doc,
            pleat
        ),
        cmd!(
            "design.gather",
            "Gather",
            ["Pattern"],
            None,
            "{id? (a path or a piece; default: the selection), from: anchor | [x, y] (document points on the outline), to: anchor | [x, y] (the edge to gather runs from one forward to the other), into?: mm (the length it gathers into, shorter than the edge)} a gathering mark: a wavy line 5 mm inside the edge and a tick across it at each end, in the piece or above the path (without from and to: between two selected points, the shorter way round) → {id, mark, mm (the edge's length), into?, ratio?}",
            has_doc,
            gather
        ),
        cmd!(
            "design.collar",
            "Draft Collar",
            ["Pattern"],
            None,
            "{back: {id (a path or a piece), from?: anchor (centre back at the neck), to?: anchor (the neck point), shoulder?: anchor (the shoulder point)}, front: {id, from?: anchor (the neck point), to?: anchor (centre front at the neck), shoulder?: anchor}, overlap?: mm (30: how far the shoulder points overlap), width?: mm (60), lower?: mm (0; 10 lowers a block's neckline for ease)} a flat (Peter Pan) collar: the front neckline laid against the back's at the neck point, the shoulders overlapping, the collar drawn the width out from the joined neckline with its front rounded to centre front; a bodice block part knows its own points (without back and front: the bodice block's back and front among the selection). The new path goes above the back → {id, neck (mm: the neck edge's length)}",
            has_doc,
            collar
        ),
        cmd!(
            "design.pocket",
            "Patch Pocket",
            ["Pattern"],
            None,
            "{width: mm, depth: mm (finished), hem?: mm (25: folded inside at the top), corners?: \"square\" (default) | \"round\" | \"point\", radius?: mm (round; a fifth of the smaller side), point?: mm (point; how far the bottom middle drops, a fifth of the width), origin?: [x, y] (document points: the piece's top-left; default 20 mm inside the first artboard), on?: id (a garment path or piece: where the finished pocket is marked), at?: [x, y] (document points: the finished pocket's top-left on it; given with on)} a patch pocket piece (its hem included, make it a pattern piece for its allowance) with its hem fold line (dashed); with on and at, the finished pocket's outline is marked on the garment (dashed, in the piece) → {id, fold, placement?}",
            has_doc,
            pocket
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
            "design.matchLength",
            "Match Seam Length",
            ["Pattern"],
            None,
            "{id? (a path or a piece; default: the selection), from: anchor, to: anchor (the seam: from one point forward to another), length?: mm, likeId?: id (a path or a piece), likeFrom?: anchor, likeTo?: anchor (another seam to match instead of a length)} true a seam: its end point (to) moves along the seam's direction until the seam is that long; the edge after it follows → {id, mm, moved (mm)}",
            has_doc,
            match_length
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
            let (i, inserted) = point_on(&mut ring, q, "to", CMD)?;
            if inserted && i <= apex {
                apex += 1;
            }
            i
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

/// The length in points along `ring` from anchor `from` forward to anchor `to`.
fn along(ring: &[Knot], from: usize, to: usize) -> f64 {
    let n = ring.len();
    let path = path_of(ring);
    let sp = &path.subpaths[0];
    let steps = (to + n - from) % n;
    (0..steps).map(|k| kurbo::ParamCurveArclen::arclen(&sp.segment((from + k) % n), 1e-9)).sum()
}

/// `from` and `to` anchors of the outline `ring`, from params named `from_key` and `to_key`.
fn seam(ring: &[Knot], p: &Value, from_key: &str, to_key: &str, cmd: &str) -> Result<(usize, usize)> {
    let (Some(from), Some(to)) = (anchor_param(p, from_key, cmd)?, anchor_param(p, to_key, cmd)?) else {
        return Err(bad(cmd, format!("{from_key} and {to_key}: point numbers along the path")));
    };
    let n = ring.len();
    if from >= n || to >= n || from == to {
        return Err(bad(cmd, format!("two different points: the path has points 0 to {}", n.saturating_sub(1))));
    }
    Ok((from, to))
}

fn length(s: &mut Session, p: &Value) -> Result<Value> {
    const CMD: &str = "design.length";
    let id = super::measure::target_path(s, p, CMD)?;
    let ring = ring_of(&s.doc()?.doc, id, CMD)?;
    let (from, to) = seam(&ring, p, "from", "to", CMD)?;
    Ok(json!({"mm": pt_to_mm(along(&ring, from, to))}))
}

fn match_length(s: &mut Session, p: &Value) -> Result<Value> {
    const CMD: &str = "design.matchLength";
    let id = super::measure::target_path(s, p, CMD)?;
    let doc = &s.doc()?.doc;
    let ring = ring_of(doc, id, CMD)?;
    let (from, to) = seam(&ring, p, "from", "to", CMD)?;
    let target = match (p.get("length"), id_param(p, "likeId")) {
        (Some(v), None) => pf_core::mm_to_pt(v.as_f64().filter(|l| l.is_finite() && *l > 0.0).ok_or_else(|| bad(CMD, "length: mm, more than 0"))?),
        (None, Some(other)) => {
            let other = super::measure::target_path(s, &json!({"id": other.0}), CMD)?;
            let theirs = ring_of(doc, other, CMD)?;
            let (f, t) = seam(&theirs, p, "likeFrom", "likeTo", CMD)?;
            along(&theirs, f, t)
        }
        _ => return Err(bad(CMD, "length (mm) or likeId with likeFrom and likeTo: the seam to match")),
    };
    // The seam's end moves along the direction the seam arrives in (its handle, or the segment
    // from the point before), with its handles: the edge after it follows.
    let n = ring.len();
    let end = ring[to];
    let back = if (end.p - end.h_in).len() > 1e-9 { end.h_in } else { ring[(to + n - 1) % n].p };
    let dir = (end.p - back).unit().ok_or_else(|| bad(CMD, "the seam's end has no direction"))?;
    let moved = |d: f64| {
        let mut r = ring.clone();
        let k = &mut r[to];
        let v = dir * d;
        *k = Knot { p: k.p + v, h_in: k.h_in + v, h_out: k.h_out + v };
        r
    };
    let error = |d: f64| along(&moved(d), from, to) - target;
    // Secant steps from no move and the length's difference (the length changes about one for
    // one with the move).
    let (mut d0, mut d1) = (0.0, target - along(&ring, from, to));
    let (mut e0, mut e1) = (error(d0), error(d1));
    for _ in 0..40 {
        if e1.abs() < 1e-9 || (e1 - e0).abs() < 1e-15 {
            break;
        }
        let d2 = d1 - e1 * (d1 - d0) / (e1 - e0);
        (d0, e0, d1) = (d1, e1, d2);
        e1 = error(d1);
    }
    if e1.abs() > 1e-6 {
        return Err(bad(CMD, "can't make the seam that long by moving its end"));
    }
    let out = moved(d1);
    s.edit("Match Seam Length", |d, sel| {
        let n = d.node(id).ok_or(EngineError::NoNode(id))?;
        let new = reshaped(n, path_of(&out));
        if let Some(m) = d.node_mut(id) {
            *m = new;
        }
        sel.set([id]);
        Ok(())
    })?;
    Ok(json!({"id": id.0, "mm": pt_to_mm(along(&out, from, to)), "moved": pt_to_mm(d1)}))
}

/// The anchor at `q` on the outline: the one there, or a new one put there (then `true`). Within
/// 5 mm of the outline.
fn point_on(ring: &mut Vec<Knot>, q: Point, key: &str, cmd: &str) -> Result<(usize, bool)> {
    let path = path_of(ring);
    let (_, seg, t, hit, dist) = path.nearest(q).ok_or_else(|| bad(cmd, "the path has no outline"))?;
    if dist > pf_core::mm_to_pt(5.0) {
        return Err(bad(cmd, format!("{key}: a point on the outline (within 5 mm of it)")));
    }
    if let Some(i) = ring.iter().position(|k| (to_point(k.p) - hit).hypot() < 1e-6) {
        return Ok((i, false));
    }
    let mut sp = path.subpaths[0].clone();
    let i = sp.insert_anchor(seg, t);
    *ring = sp.anchors.iter().map(|a| Knot { p: to_pt(a.p), h_in: to_pt(a.h_in), h_out: to_pt(a.h_out) }).collect();
    Ok((i, true))
}

/// An anchor param: a point's number, or `[x, y]` (document points) on the outline.
fn place(ring: &mut Vec<Knot>, p: &Value, key: &str, cmd: &str) -> Result<(usize, bool)> {
    match p.get(key) {
        Some(Value::Array(_)) => {
            let [x, y] = finite_numbers::<2>(&p[key]).ok_or_else(|| bad(cmd, format!("{key}: [x, y] in document points")))?;
            point_on(ring, Point::new(x, y), key, cmd)
        }
        _ => {
            let i = anchor_param(p, key, cmd)?.ok_or_else(|| bad(cmd, format!("{key}: a point's number along the path, or [x, y] on it")))?;
            if i >= ring.len() {
                return Err(bad(cmd, format!("the path has points 0 to {}", ring.len().saturating_sub(1))));
            }
            Ok((i, false))
        }
    }
}

fn spread(s: &mut Session, p: &Value) -> Result<Value> {
    const CMD: &str = "design.spread";
    let id = super::measure::target_path(s, p, CMD)?;
    let mut ring = ring_of(&s.doc()?.doc, id, CMD)?;
    let amount = p.get("amount").and_then(Value::as_f64).ok_or_else(|| bad(CMD, "amount: how far it opens, mm"))?;
    let how = match str_param(p, "mode") {
        None | Some("hinge") => pf_core::design::Spread::Hinge,
        Some("parallel") => pf_core::design::Spread::Parallel,
        Some(o) => return Err(bad(CMD, format!("mode: \"hinge\" or \"parallel\", not {o:?}"))),
    };
    let (mut from, _) = place(&mut ring, p, "from", CMD)?;
    let (to, inserted) = place(&mut ring, p, "to", CMD)?;
    if inserted && to <= from {
        from += 1;
    }
    let out = pf_core::design::spread(&ring, from, to, pf_core::mm_to_pt(amount), how).map_err(|e| err(CMD, e))?;
    let count = out.len();
    s.edit("Slash and Spread", |d, sel| {
        let n = d.node(id).ok_or(EngineError::NoNode(id))?;
        let new = reshaped(n, path_of(&out));
        if let Some(m) = d.node_mut(id) {
            *m = new;
        }
        sel.set([id]);
        Ok(())
    })?;
    Ok(json!({"id": id.0, "anchors": count}))
}

fn facing(s: &mut Session, p: &Value) -> Result<Value> {
    const CMD: &str = "design.facing";
    let id = super::measure::target_path(s, p, CMD)?;
    let doc = &s.doc()?.doc;
    let mut ring = ring_of(doc, id, CMD)?;
    let width = match p.get("width") {
        None | Some(Value::Null) => 50.0,
        Some(v) => v.as_f64().filter(|w| w.is_finite() && *w > 0.0).ok_or_else(|| bad(CMD, "width: mm, more than 0"))?,
    };
    let (from, to) = if p.get("from").is_none() && p.get("to").is_none() {
        // Two selected points: the shorter way round between them.
        let [a, b] = selected_points(s, id)?[..] else {
            return Err(bad(CMD, "select two points of one path: the ends of the edge to face"));
        };
        if along(&ring, a, b) <= along(&ring, b, a) { (a, b) } else { (b, a) }
    } else {
        let (mut from, _) = place(&mut ring, p, "from", CMD)?;
        let (to, inserted) = place(&mut ring, p, "to", CMD)?;
        if inserted && to <= from {
            from += 1;
        }
        (from, to)
    };
    let out = pf_core::design::facing(&ring, from, to, pf_core::mm_to_pt(width)).map_err(|e| err(CMD, e))?;
    // Named after the piece, or the path; put above the piece (or the path).
    let group = doc.parent_of(id).filter(|_| in_piece(doc, id));
    let base = match group.and_then(|g| doc.node(g)).and_then(|g| g.extras().pattern.as_deref().and_then(pf_core::PatternTag::piece)) {
        Some(piece) if !piece.name.is_empty() => Some(piece.name.clone()),
        _ => doc.node(id).and_then(|n| n.name.clone()),
    };
    let name = base.map_or_else(|| "Facing".to_string(), |b| format!("{b} Facing"));
    let anchor = group.unwrap_or(id);
    let count = out.len();
    let new = s.edit("Make Facing", |d, sel| {
        let fid = d.alloc_id();
        let mut n = Node::path(
            fid,
            path_of(&out),
            vectorcraft_doc::Appearance::basic(vectorcraft_color::Paint::None, vectorcraft_color::Paint::solid(vectorcraft_color::Color::BLACK), 1.0),
        );
        n.name = Some(name);
        d.insert_beside(vec![(anchor, vec![n])], false)?;
        sel.set([fid]);
        Ok(fid)
    })?;
    Ok(json!({"id": new.0, "anchors": count}))
}

fn pleat(s: &mut Session, p: &Value) -> Result<Value> {
    const CMD: &str = "design.pleat";
    let id = super::measure::target_path(s, p, CMD)?;
    let doc = &s.doc()?.doc;
    let mut ring = ring_of(doc, id, CMD)?;
    let depth = p
        .get("depth")
        .and_then(Value::as_f64)
        .filter(|d| d.is_finite() && *d > 0.0)
        .ok_or_else(|| bad(CMD, "depth: how deep the pleat folds under, mm"))?;
    let (mut from, _) = place(&mut ring, p, "from", CMD)?;
    let (to, inserted) = place(&mut ring, p, "to", CMD)?;
    if inserted && to <= from {
        from += 1;
    }
    let kind = match str_param(p, "kind") {
        None | Some("knife") => pf_core::design::PleatKind::Knife,
        Some("inverted") => pf_core::design::PleatKind::Inverted,
        Some(o) => return Err(bad(CMD, format!("kind: \"knife\" or \"inverted\", not {o:?}"))),
    };
    let (out, marks) = pf_core::design::pleat(&ring, from, to, pf_core::mm_to_pt(depth), kind, pf_core::mm_to_pt(4.0)).map_err(|e| err(CMD, e))?;
    // The marks go in the piece (they move with it) or above the path.
    let group = doc.parent_of(id).filter(|_| in_piece(doc, id));
    let count = out.len();
    let ids = s.edit("Add Pleat", |d, sel| {
        let n = d.node(id).ok_or(EngineError::NoNode(id))?;
        let new = reshaped(n, path_of(&out));
        if let Some(m) = d.node_mut(id) {
            *m = new;
        }
        let line = |pts: &[Pt]| SubPath::polyline(&pts.iter().map(|q| to_point(*q)).collect::<Vec<_>>(), false);
        let black = || vectorcraft_color::Paint::solid(vectorcraft_color::Color::BLACK);
        let fold_id = d.alloc_id();
        let mut fold = Node::path(
            fold_id,
            PathData::new(
                marks.folds.iter().map(|f| line(f)).chain(marks.arrows.iter().flat_map(|[shaft, head]| [line(shaft), line(head)])).collect(),
            ),
            vectorcraft_doc::Appearance::basic(vectorcraft_color::Paint::None, black(), 0.75),
        );
        fold.name = Some(if marks.folds.len() > 1 { "Pleat fold lines" } else { "Pleat fold line" }.into());
        let place_id = d.alloc_id();
        let mut dashed = vectorcraft_doc::Appearance::basic(vectorcraft_color::Paint::None, black(), 0.75);
        if let Some(st) = dashed.stroke_mut() {
            st.dash = Some(vectorcraft_doc::Dash { pattern: vec![4.0, 2.0], ..Default::default() });
        }
        let mut placement = Node::path(place_id, PathData::new(marks.placements.iter().map(|l| line(l)).collect()), dashed);
        placement.name = Some("Pleat placement line".into());
        match group {
            Some(g) => {
                d.insert(Some(g), usize::MAX, placement)?;
                d.insert(Some(g), usize::MAX, fold)?;
            }
            None => d.insert_beside(vec![(id, vec![placement, fold])], false)?,
        }
        sel.set([id]);
        Ok([fold_id, place_id])
    })?;
    Ok(json!({"id": id.0, "anchors": count, "fold": ids[0].0, "placement": ids[1].0}))
}

fn gather(s: &mut Session, p: &Value) -> Result<Value> {
    const CMD: &str = "design.gather";
    let id = super::measure::target_path(s, p, CMD)?;
    let doc = &s.doc()?.doc;
    let mut ring = ring_of(doc, id, CMD)?;
    let into = match p.get("into") {
        None | Some(Value::Null) => None,
        Some(v) => Some(v.as_f64().filter(|l| l.is_finite() && *l > 0.0).ok_or_else(|| bad(CMD, "into: the length it gathers into, mm"))?),
    };
    let (from, to) = if p.get("from").is_none() && p.get("to").is_none() {
        // Two selected points: the shorter way round between them.
        let [a, b] = selected_points(s, id)?[..] else {
            return Err(bad(CMD, "select two points of one path: the ends of the edge to gather"));
        };
        if along(&ring, a, b) <= along(&ring, b, a) { (a, b) } else { (b, a) }
    } else {
        let (mut from, _) = place(&mut ring, p, "from", CMD)?;
        let (to, inserted) = place(&mut ring, p, "to", CMD)?;
        if inserted && to <= from {
            from += 1;
        }
        (from, to)
    };
    let length = pt_to_mm(along(&ring, from, to));
    if let Some(l) = into
        && l >= length
    {
        return Err(bad(CMD, format!("into: shorter than the edge ({length:.1} mm), or there is nothing to gather")));
    }
    let lines = pf_core::design::gather_marks(&ring, from, to, pf_core::mm_to_pt(5.0), pf_core::mm_to_pt(8.0)).map_err(|e| err(CMD, e))?;
    let group = doc.parent_of(id).filter(|_| in_piece(doc, id));
    let inserted_points = ring.len() != ring_of(doc, id, CMD)?.len();
    let mark = s.edit("Gather", |d, sel| {
        if inserted_points {
            // The points the edge ends at were put on the outline.
            let n = d.node(id).ok_or(EngineError::NoNode(id))?;
            let new = reshaped(n, path_of(&ring));
            if let Some(m) = d.node_mut(id) {
                *m = new;
            }
        }
        let line = |pts: &[Pt]| SubPath::polyline(&pts.iter().map(|q| to_point(*q)).collect::<Vec<_>>(), false);
        let black = vectorcraft_color::Paint::solid(vectorcraft_color::Color::BLACK);
        let mid = d.alloc_id();
        let mut n = Node::path(
            mid,
            PathData::new(lines.iter().map(|l| line(l)).collect()),
            vectorcraft_doc::Appearance::basic(vectorcraft_color::Paint::None, black, 0.75),
        );
        n.name = Some(match into {
            Some(l) => format!("Gather to {l:.0} mm"),
            None => "Gather".into(),
        });
        match group {
            Some(g) => d.insert(Some(g), usize::MAX, n)?,
            None => d.insert_beside(vec![(id, vec![n])], false).map(|_| mid)?,
        };
        sel.set([id]);
        Ok(mid)
    })?;
    let mut r = json!({"id": id.0, "mark": mark.0, "mm": length});
    if let Some(l) = into {
        r["into"] = json!(l);
        r["ratio"] = json!(length / l);
    }
    Ok(r)
}

/// One side's neckline for `design.collar`: the path, its neckline's ends and the shoulder
/// point. A bodice block part that is still the block knows its own.
fn neckline_param(s: &Session, p: &Value, key: &str, fallback: Option<NodeId>, cmd: &str) -> Result<(NodeId, Vec<Knot>, usize, usize, usize)> {
    let given;
    let v = match (p.get(key), fallback) {
        (Some(v), _) => v,
        (None, Some(id)) => {
            given = json!({"id": id.0});
            &given
        }
        (None, None) => return Err(bad(cmd, format!("{key}: {{id, from?, to?, shoulder?}} (or select the bodice block's back and front)"))),
    };
    let id = super::measure::target_path(s, v, cmd)?;
    let doc = &s.doc()?.doc;
    let ring = ring_of(doc, id, cmd)?;
    let block = doc.node(id).and_then(|n| n.extras().block.as_deref().cloned());
    let known = match block {
        Some(b) if b.kind == pf_core::block::BlockKind::Bodice && b.part == 0 => Some((1, 2, 6)),
        Some(b) if b.kind == pf_core::block::BlockKind::Bodice && b.part == 1 => Some((12, 0, 9)),
        _ => None,
    };
    let get = |k: &str, d: Option<usize>| -> Result<usize> {
        match (anchor_param(v, k, cmd)?, d) {
            (Some(i), _) | (None, Some(i)) => Ok(i),
            (None, None) => Err(bad(cmd, format!("{key}.{k}: a point's number along the path (only a bodice block part knows its own)"))),
        }
    };
    let (from, to, shoulder) = (get("from", known.map(|k| k.0))?, get("to", known.map(|k| k.1))?, get("shoulder", known.map(|k| k.2))?);
    Ok((id, ring, from, to, shoulder))
}

fn collar(s: &mut Session, p: &Value) -> Result<Value> {
    const CMD: &str = "design.collar";
    let mm = |k: &str, d: f64, zero_ok: bool| -> Result<f64> {
        match p.get(k) {
            None | Some(Value::Null) => Ok(d),
            Some(v) => v.as_f64().filter(|x| x.is_finite() && (*x > 0.0 || zero_ok && *x == 0.0)).ok_or_else(|| bad(CMD, format!("{k}: mm"))),
        }
    };
    let (overlap, width, lower) = (mm("overlap", 30.0, true)?, mm("width", 60.0, false)?, mm("lower", 0.0, true)?);
    // Without back and front: the bodice block's back and front among the selection.
    let (mut sel_back, mut sel_front) = (None, None);
    if p.get("back").is_none() && p.get("front").is_none() {
        let st = s.doc()?;
        for id in st.selection.in_paint_order(&st.doc) {
            let Ok(path) = super::measure::target_path(s, &json!({"id": id.0}), CMD) else { continue };
            match st.doc.node(path).and_then(|n| n.extras().block.as_deref().cloned()) {
                Some(b) if b.kind == pf_core::block::BlockKind::Bodice && b.part == 0 => sel_back = Some(path),
                Some(b) if b.kind == pf_core::block::BlockKind::Bodice && b.part == 1 => sel_front = Some(path),
                _ => {}
            }
        }
    }
    let (back_id, back, bf, bt, bs) = neckline_param(s, p, "back", sel_back, CMD)?;
    let (_, front, ff, ft, fs) = neckline_param(s, p, "front", sel_front, CMD)?;
    let c = pf_core::design::flat_collar(
        pf_core::design::Neckline { ring: &back, from: bf, to: bt, shoulder: bs },
        pf_core::design::Neckline { ring: &front, from: ff, to: ft, shoulder: fs },
        pf_core::mm_to_pt(overlap),
        pf_core::mm_to_pt(width),
        pf_core::mm_to_pt(lower),
    )
    .map_err(|e| err(CMD, e))?;
    let doc = &s.doc()?.doc;
    // Drafted where the back is: above it, or above its piece.
    let anchor = doc.parent_of(back_id).filter(|_| in_piece(doc, back_id)).unwrap_or(back_id);
    let new = s.edit("Draft Collar", |d, sel| {
        let id = d.alloc_id();
        let mut n = Node::path(
            id,
            path_of(&c.outline),
            vectorcraft_doc::Appearance::basic(vectorcraft_color::Paint::None, vectorcraft_color::Paint::solid(vectorcraft_color::Color::BLACK), 1.0),
        );
        n.name = Some("Collar".into());
        d.insert_beside(vec![(anchor, vec![n])], false)?;
        sel.set([id]);
        Ok(id)
    })?;
    Ok(json!({"id": new.0, "neck": pt_to_mm(c.neck_length)}))
}

fn pocket(s: &mut Session, p: &Value) -> Result<Value> {
    const CMD: &str = "design.pocket";
    let mm = |k: &str, d: Option<f64>| -> Result<f64> {
        match (p.get(k), d) {
            (None | Some(Value::Null), Some(d)) => Ok(d),
            (None | Some(Value::Null), None) => Err(bad(CMD, format!("{k}: mm"))),
            (Some(v), _) => v.as_f64().filter(|x| x.is_finite() && *x >= 0.0).ok_or_else(|| bad(CMD, format!("{k}: mm"))),
        }
    };
    let (width, depth, hem) = (mm("width", None)?, mm("depth", None)?, mm("hem", Some(25.0))?);
    let corners = match str_param(p, "corners") {
        None | Some("square") => pf_core::design::PocketCorners::Square,
        Some("round") => pf_core::design::PocketCorners::Round(mm("radius", Some(width.min(depth) / 5.0))?),
        Some("point") => pf_core::design::PocketCorners::Point(mm("point", Some(width / 5.0))?),
        Some(o) => return Err(bad(CMD, format!("corners: \"square\", \"round\" or \"point\", not {o:?}"))),
    };
    let pk = pf_core::design::patch_pocket(width, depth, hem, corners).map_err(|e| err(CMD, e))?;
    let doc = &s.doc()?.doc;
    let point = |k: &str| -> Result<Option<Point>> {
        match p.get(k) {
            None | Some(Value::Null) => Ok(None),
            Some(v) => {
                let [x, y] = finite_numbers::<2>(v).ok_or_else(|| bad(CMD, format!("{k}: [x, y] in document points")))?;
                Ok(Some(Point::new(x, y)))
            }
        }
    };
    let origin = point("origin")?.unwrap_or_else(|| {
        let o = doc.artboards.first().map_or(Point::ZERO, |a| a.rect.origin());
        Point::new(o.x + pf_core::mm_to_pt(20.0), o.y + pf_core::mm_to_pt(20.0))
    });
    // The placement mark goes on a garment piece (or above a path), its top-left at `at`.
    let place = match (id_param(p, "on"), point("at")?) {
        (Some(on), Some(at)) => {
            let target = super::measure::target_path(s, &json!({"id": on.0}), CMD)?;
            let group = doc.parent_of(target).filter(|_| in_piece(doc, target));
            Some((group, target, at))
        }
        (None, None) => None,
        _ => return Err(bad(CMD, "on and at go together: the garment piece and where the pocket's top-left corner goes on it")),
    };
    let at_mm = |q: Pt, o: Point| to_pt(Point::new(o.x + pf_core::mm_to_pt(q.x), o.y + pf_core::mm_to_pt(q.y)));
    let moved = |ks: &[Knot], o: Point| -> Vec<Knot> {
        ks.iter().map(|k| Knot { p: at_mm(k.p, o), h_in: at_mm(k.h_in, o), h_out: at_mm(k.h_out, o) }).collect()
    };
    let piece = moved(&pk.piece, origin);
    let fold = [at_mm(pk.fold[0], origin), at_mm(pk.fold[1], origin)];
    let parent = s.doc()?.insertion_parent();
    let ids = s.edit("Patch Pocket", |d, sel| {
        let black = || vectorcraft_color::Paint::solid(vectorcraft_color::Color::BLACK);
        let line = |pts: &[Pt]| SubPath::polyline(&pts.iter().map(|q| to_point(*q)).collect::<Vec<_>>(), false);
        let dashed = || {
            let mut a = vectorcraft_doc::Appearance::basic(vectorcraft_color::Paint::None, black(), 0.75);
            if let Some(st) = a.stroke_mut() {
                st.dash = Some(vectorcraft_doc::Dash { pattern: vec![4.0, 2.0], ..Default::default() });
            }
            a
        };
        let pid = d.alloc_id();
        let mut n = Node::path(pid, path_of(&piece), vectorcraft_doc::Appearance::basic(vectorcraft_color::Paint::None, black(), 1.0));
        n.name = Some("Patch Pocket".into());
        d.insert(parent, usize::MAX, n)?;
        let fid = d.alloc_id();
        let mut f = Node::path(fid, PathData::single(line(&fold)), dashed());
        f.name = Some("Pocket hem fold line".into());
        d.insert(parent, usize::MAX, f)?;
        let mut mark = None;
        if let Some((group, target, at)) = place {
            let mid = d.alloc_id();
            let mut m = Node::path(mid, path_of(&moved(&pk.finished, at)), dashed());
            m.name = Some("Pocket placement".into());
            match group {
                Some(g) => {
                    d.insert(Some(g), usize::MAX, m)?;
                }
                None => d.insert_beside(vec![(target, vec![m])], false)?,
            }
            mark = Some(mid);
        }
        sel.set([pid]);
        Ok((pid, fid, mark))
    })?;
    let mut r = json!({"id": ids.0.0, "fold": ids.1.0});
    if let Some(m) = ids.2 {
        r["placement"] = json!(m.0);
    }
    Ok(r)
}
