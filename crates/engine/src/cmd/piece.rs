//! Pattern pieces (PF-1): a closed path becomes a sewing-pattern piece.
//!
//! A piece is a group holding the path the user drew, its **seam line**, with the piece's data
//! ([`pf_core::Piece`]: name, cut count, fabric, seam allowance per edge, corner styles, notches,
//! grainline) on the group. The cutting line, the notches, the grainline and the label are
//! generated from the seam line as locked children, so they print and export like any art, and
//! [`refresh`] keeps them in step whenever the seam line or the piece's data changes (as linked
//! dimensions follow the art they measure). The seam line's segments are the piece's edges, numbered
//! from 0 in drawing order.
//!
//! Lengths come and go in millimetres (a number; or a string with a unit, `"5/8 in"`); the
//! document is in points.
use super::*;
use pf_core::{Corner, Mark, Notch, NotchKind, Outline, PatternTag, Piece, Pt, mm_to_pt, pt_to_mm};
use serde_json::{Value, json};
use std::sync::Arc;
use vectorcraft_color::{Color, Paint};
use vectorcraft_doc::{Appearance, CharStyle, Document, Node, NodeId, NodeKind, TextObject};
use vectorcraft_geom::{Affine, BezPath, PathData, Point, SubPath};

/// Most pieces one command makes or changes.
const MAX_PIECES: usize = 1000;
/// How closely curves of the seam line are followed, in points (a fiftieth of a millimetre is
/// below what a printer or a cutter can show).
pub(crate) const FLATTEN_PT: f64 = 0.05;
/// The label's type size, in points.
const LABEL_PT: f64 = 10.0;

pub fn specs() -> Vec<CommandSpec> {
    vec![
        cmd!(
            "piece.make",
            "Make Pattern Piece",
            ["Pattern"],
            None,
            "{ids? (default: the selection), allowance?: mm (10; or a length with its unit, \"5/8 in\"), name?} each closed path becomes a pattern piece: a group of its seam line (the path) with its cutting line, grainline and label generated → {ids}",
            has_selection,
            make
        ),
        cmd!(
            "piece.release",
            "Release Pattern Piece",
            ["Pattern"],
            None,
            "{ids? (default: the selected pieces)} each piece goes back to its seam line, a plain path; the generated marks are deleted → {ids}",
            has_selection,
            release
        ),
        cmd!(
            "piece.set",
            "Piece Options",
            [],
            None,
            "{id? (default: the selected piece), name?, cut?: 1..999, mirror?: bool (cut in mirror-image pairs), fabric?, allowance?: mm (every edge without its own), edges?: {\"<edge>\": mm | null (back to the piece's allowance)}, corners?: {\"<corner>\": \"mitre\" | \"square\"} (corner i is where edge i-1 meets edge i), grainline?: {from: [x, y], to: [x, y]} (document points) | \"auto\" (vertical through the middle), fold?: edge | \"auto\" (the longest straight edge) | null (the straight edge cut on the fold: no allowance, laid against the fabric's fold or opened out by marker.make)} → piece.info",
            has_doc,
            set
        ),
        cmd!(
            "piece.addNotch",
            "Add Notch",
            [],
            None,
            "{id?, edge: 0.., at?: 0..1 (0.5; along the edge by length), kind?: \"slit\" (default) | \"double\" | \"v\"} → piece.info",
            has_doc,
            add_notch
        ),
        cmd!(
            "piece.removeNotch",
            "Remove Notch",
            [],
            None,
            "{id?, index: the notch's place in piece.info's notches} → piece.info",
            has_doc,
            remove_notch
        ),
        cmd!(
            query "piece.info",
            "Piece Info",
            [],
            None,
            "{id? (default: the selected piece)} → {id, seam (the seam line's id), name, cut, mirror, fabric, allowance, edges, corners, notches, grainline, edgeCount, seamLength (mm), area (cm², inside the seam line), problem (why there is no cutting line, if there is none)}",
            has_doc,
            info
        ),
        cmd!(query "piece.list", "Pattern Pieces", [], None, "{} the document's pieces, in paint order → {pieces: [{id, name, cut, mirror, fabric}]}", has_doc, list),
    ]
}

fn err(cmd: &str, e: impl ToString) -> EngineError {
    bad(cmd, e.to_string())
}

/// The piece on `n`, if it is a piece's group.
pub(crate) fn piece_on(n: &Node) -> Option<&Piece> {
    n.extras().pattern.as_deref().and_then(PatternTag::piece)
}

/// Which part of a piece `n` draws, if it is one.
pub(crate) fn mark_of(n: &Node) -> Option<Mark> {
    n.extras().pattern.as_deref().and_then(PatternTag::mark)
}

/// The piece group `id` is or is in (one of its parts).
pub(crate) fn piece_group(doc: &Document, id: NodeId) -> Option<NodeId> {
    let n = doc.node(id)?;
    if piece_on(n).is_some() {
        return Some(id);
    }
    let parent = doc.parent_of(id)?;
    doc.node(parent).and_then(piece_on).map(|_| parent)
}

/// The pieces a command acts on: `ids`/`id` in the params, else the selected pieces (or pieces
/// with a selected part).
pub(crate) fn target_pieces(s: &Session, p: &Value, cmd: &str) -> Result<Vec<NodeId>> {
    let st = s.doc()?;
    let given: Vec<NodeId> = if let Some(id) = id_param(p, "id") {
        vec![id]
    } else if p.get("ids").is_some() {
        checked_ids_param(s, p, "ids", cmd)?
    } else {
        st.selection.in_paint_order(&st.doc)
    };
    let mut out: Vec<NodeId> = Vec::new();
    for id in given {
        if let Some(g) = piece_group(&st.doc, id)
            && !out.contains(&g)
        {
            out.push(g);
        }
    }
    if out.is_empty() {
        return Err(bad(cmd, "select a pattern piece (Pattern › Make Pattern Piece makes one)"));
    }
    if out.len() > MAX_PIECES {
        return Err(bad(cmd, "too many pieces at once"));
    }
    Ok(out)
}

fn one_piece(s: &Session, p: &Value, cmd: &str) -> Result<NodeId> {
    let ids = target_pieces(s, p, cmd)?;
    match ids.as_slice() {
        [one] => Ok(*one),
        _ => Err(bad(cmd, "select one pattern piece")),
    }
}

/// A length param in millimetres: a number (mm) or a string with a unit (`"5/8 in"`, `"1 cm"`).
pub(crate) fn mm_param(v: &Value) -> Option<f64> {
    match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => vectorcraft_doc::Unit::Millimeters.parse(s).map(pt_to_mm),
        _ => None,
    }
    .filter(|v| v.is_finite())
}

fn allowance_param(v: &Value, cmd: &str) -> Result<f64> {
    let mm = mm_param(v).ok_or_else(|| bad(cmd, "a seam allowance is a number of mm, or a length with its unit (\"5/8 in\")"))?;
    pf_core::piece::check_allowance(mm).map_err(|e| err(cmd, e))
}

/// The seam line's first closed subpath, flattened into an outline in document points: segment
/// `i` of the subpath is edge `i`. Also the number of edges.
pub(crate) fn outline_of(path: &PathData) -> Option<(Outline, usize)> {
    let sp: &SubPath = path.subpaths.iter().find(|sp| sp.closed)?;
    let edges = sp.segment_count();
    let mut out = Outline::default();
    for i in 0..edges {
        let seg = sp.segment(i);
        out.points.push(Pt::new(seg.p0.x, seg.p0.y));
        out.edge_of.push(i);
        if sp.segment_is_line(i) {
            continue;
        }
        let mut bez = BezPath::new();
        bez.move_to(seg.p0);
        bez.curve_to(seg.p1, seg.p2, seg.p3);
        let mut pts: Vec<Point> = Vec::new();
        vectorcraft_geom::kurbo::flatten(bez, FLATTEN_PT, |el| match el {
            vectorcraft_geom::kurbo::PathEl::LineTo(p) => pts.push(p),
            vectorcraft_geom::kurbo::PathEl::MoveTo(_) | vectorcraft_geom::kurbo::PathEl::ClosePath => {}
            _ => {}
        });
        // The last point is the next segment's start.
        pts.pop();
        for p in pts {
            out.points.push(Pt::new(p.x, p.y));
            out.edge_of.push(i);
        }
        if out.points.len() > pf_core::allowance::MAX_POINTS {
            return None;
        }
    }
    Some((out, edges))
}

fn polyline(points: &[Pt], closed: bool) -> SubPath {
    SubPath::polyline(&points.iter().map(|p| Point::new(p.x, p.y)).collect::<Vec<_>>(), closed)
}

fn black() -> Paint {
    Paint::solid(Color::BLACK)
}

/// The seam line's look: a thin dashed line, no fill.
fn seam_look() -> Appearance {
    let mut a = Appearance::basic(Paint::None, black(), 0.5);
    if let Some(st) = a.stroke_mut() {
        st.dash = Some(vectorcraft_doc::Dash { pattern: vec![4.0, 2.0], ..Default::default() });
    }
    a
}

/// A generated part: locked (it is drawn from the seam line, not edited), tagged with its role.
fn mark_node(id: NodeId, mark: Mark, kind: NodeKind, appearance: Appearance) -> Node {
    let mut n = Node::new(id, kind);
    n.appearance = appearance.into();
    n.locked = true;
    n.name = Some(
        match mark {
            Mark::Seam => "Seam line",
            Mark::Cut => "Cutting line",
            Mark::Notches => "Notches",
            Mark::Grainline => "Grainline",
            Mark::Label => "Label",
        }
        .into(),
    );
    n.extras_mut().pattern = Some(Box::new(PatternTag::Mark(mark)));
    n
}

fn path_kind(subpaths: Vec<SubPath>) -> NodeKind {
    NodeKind::Path { path: PathData::new(subpaths), rule: Default::default(), live: None, clipping: false, guide: false }
}

/// Why `piece` on `seam` has no cutting line, if it has none.
fn problem(seam: Option<&Node>) -> Option<String> {
    let Some(seam) = seam else { return Some("the piece has no seam line".into()) };
    let NodeKind::Path { path, .. } = &seam.kind else { return Some("the seam line is not a path".into()) };
    let Some((outline, _)) = outline_of(path) else { return Some("the seam line is not a closed path".into()) };
    pf_core::cutting_line(&outline, |_| 0.0, |_| Corner::Mitre).err().map(|e| e.to_string())
}

/// Draw the parts of the piece group `id` again from its seam line and data, keeping their ids.
fn rebuild(doc: &mut Document, id: NodeId) -> Result<()> {
    let Some(group) = doc.node(id) else { return Ok(()) };
    let Some(mut piece) = piece_on(group).cloned() else { return Ok(()) };
    let children: Vec<Arc<Node>> = group.children().cloned().unwrap_or_default();
    let seam = children.iter().find(|c| mark_of(c) == Some(Mark::Seam)).cloned();
    let old = |m: Mark| children.iter().find(|c| mark_of(c) == Some(m)).map(|c| c.id);
    // Children that are neither the seam line nor a generated part (added by the user): kept, above.
    let others: Vec<Arc<Node>> = children.iter().filter(|c| mark_of(c).is_none()).cloned().collect();
    let mut parts: Vec<Arc<Node>> = Vec::new();
    let mm = mm_to_pt(1.0);
    let geometry = seam.as_ref().and_then(|s| match &s.kind {
        NodeKind::Path { path, .. } => outline_of(path),
        _ => None,
    });
    let ids = |m: Mark, doc: &mut Document| old(m).unwrap_or_else(|| doc.alloc_id());
    if let Some((outline, edges)) = &geometry {
        piece.fit_edges(*edges);
        if let Ok(cut) = pf_core::cutting_line(outline, |e| mm_to_pt(piece.allowance_of(e)), |c| piece.corner_of(c)) {
            let a = Appearance::basic(Paint::None, black(), 1.0);
            parts.push(Arc::new(mark_node(ids(Mark::Cut, doc), Mark::Cut, path_kind(vec![polyline(&cut, true)]), a)));
        }
    }
    if let Some(s) = &seam {
        parts.push(s.clone());
    }
    if let Some((outline, _)) = &geometry {
        let grain = piece.grainline.or_else(|| pf_core::marks::default_grainline(outline));
        if let Some(g) = grain {
            let lines = pf_core::marks::grainline(&g, mm);
            if !lines.is_empty() {
                let a = Appearance::basic(Paint::None, black(), 0.75);
                let sub = lines.iter().map(|l| polyline(l, false)).collect();
                parts.push(Arc::new(mark_node(ids(Mark::Grainline, doc), Mark::Grainline, path_kind(sub), a)));
            }
        }
        let notches: Vec<SubPath> = piece
            .notches
            .iter()
            .flat_map(|n| pf_core::marks::notch(outline, n, mm_to_pt(piece.allowance_of(n.edge)), mm))
            .map(|l| polyline(&l, false))
            .collect();
        if !notches.is_empty() {
            let a = Appearance::basic(Paint::None, black(), 1.0);
            parts.push(Arc::new(mark_node(ids(Mark::Notches, doc), Mark::Notches, path_kind(notches), a)));
        }
        // The label, beside the grainline's middle, on the side of the piece with more room.
        let at = grain.map(|g| (g.from + g.to) * 0.5).unwrap_or_default();
        let (left, right) = outline.points.iter().fold((at.x, at.x), |(l, r), p| (l.min(p.x), r.max(p.x)));
        let side = if at.x - left > right - at.x { -1.0 } else { 1.0 };
        let style = CharStyle { size: LABEL_PT, fill: black(), ..Default::default() };
        let mut text = TextObject::point(Point::ZERO, &piece.label(), style);
        text.para.justify = vectorcraft_doc::Justify::Center;
        super::typecmd::refresh_bounds(&mut text);
        let bounds = text.cached_bounds.unwrap_or_else(|| text.estimate_bounds());
        let shift = Point::new(at.x + side * (bounds.width() / 2.0 + 3.0 * mm), at.y);
        text.xf = Affine::translate(shift.to_vec2() - bounds.center().to_vec2());
        let a = Appearance::basic(black(), Paint::None, 0.0);
        parts.push(Arc::new(mark_node(ids(Mark::Label, doc), Mark::Label, NodeKind::Text(Box::new(text)), a)));
    }
    parts.extend(others);
    if let Some(n) = doc.node_mut(id) {
        n.name = Some(if piece.name.is_empty() { "Pattern piece".into() } else { piece.name.clone() });
        n.kind = NodeKind::Group { children: parts, clip: false };
        n.extras_mut().pattern = Some(Box::new(PatternTag::Piece(Box::new(piece))));
    }
    Ok(())
}

/// Called inside every edit: pieces whose seam line or data changed are drawn again, in the same
/// undo step. Returns at once in documents without pieces.
pub(crate) fn refresh(before: &Document, doc: &mut Document) -> Result<()> {
    if !doc.pieces {
        return Ok(());
    }
    let mut pieces = Vec::new();
    doc.walk(|n| {
        if piece_on(n).is_some() {
            pieces.push(n.id);
        }
    });
    doc.pieces = !pieces.is_empty();
    for id in pieces {
        let (Some(now), then) = (doc.node(id), before.node(id)) else { continue };
        if then.is_some_and(|t| t == now) {
            continue;
        }
        rebuild(doc, id)?;
    }
    Ok(())
}

fn ids_json(ids: &[NodeId]) -> Value {
    json!({ "ids": ids.iter().map(|i| i.0).collect::<Vec<_>>() })
}

fn make(s: &mut Session, p: &Value) -> Result<Value> {
    const CMD: &str = "piece.make";
    let allowance = match p.get("allowance") {
        Some(v) => allowance_param(v, CMD)?,
        None => pf_core::piece::DEFAULT_ALLOWANCE_MM,
    };
    let name = match str_param(p, "name") {
        Some(n) => pf_core::piece::check_name(n).map_err(|e| err(CMD, e))?,
        None => String::new(),
    };
    // Given ids are taken as the same objects selected would be (an object inside a listed group
    // stands for nothing more than the group).
    let roots = if p.get("ids").is_some() {
        let ids = checked_ids_param(s, p, "ids", CMD)?;
        let st = s.doc()?;
        super::edit::roots_of(&st.doc, ids)
    } else {
        super::edit::selected_roots(s)?
    };
    if roots.len() > MAX_PIECES {
        return Err(bad(CMD, "too many objects at once"));
    }
    let ids = s.edit("Make Pattern Piece", |d, sel| {
        let mut made = Vec::new();
        let mut beside = Vec::new();
        for id in roots {
            let Some(n) = d.node(id) else { continue };
            if piece_group(d, id).is_some() {
                continue;
            }
            let NodeKind::Path { path, .. } = &n.kind else { continue };
            if !path.subpaths.iter().any(|sp| sp.closed) {
                continue;
            }
            let mut seam = n.clone();
            seam.extras_mut().pattern = Some(Box::new(PatternTag::Mark(Mark::Seam)));
            // Patterns draw the seam (stitching) line dashed and the cutting line solid.
            seam.appearance = seam_look().into();
            let gid = d.alloc_id();
            let mut group = Node::group(gid, vec![Arc::new(seam)]);
            let piece = Piece { name: name.clone(), allowance, ..Piece::default() };
            group.extras_mut().pattern = Some(Box::new(PatternTag::Piece(Box::new(piece))));
            beside.push((id, vec![group]));
            made.push(gid);
        }
        if made.is_empty() {
            return Err(bad(CMD, "select closed paths: each becomes a piece, the path its seam line"));
        }
        d.insert_beside(beside, true)?;
        d.pieces = true;
        for &g in &made {
            rebuild(d, g)?;
        }
        sel.set(made.iter().copied());
        Ok(made)
    })?;
    Ok(ids_json(&ids))
}

fn release(s: &mut Session, p: &Value) -> Result<Value> {
    const CMD: &str = "piece.release";
    let pieces = target_pieces(s, p, CMD)?;
    let ids = s.edit("Release Pattern Piece", |d, sel| {
        let mut out = Vec::new();
        let mut beside = Vec::new();
        for g in pieces {
            let Some(group) = d.node(g) else { continue };
            let children = group.children().cloned().unwrap_or_default();
            let mut keep: Vec<Node> = Vec::new();
            for c in children {
                match mark_of(&c) {
                    Some(Mark::Seam) => {
                        let mut seam = (*c).clone();
                        seam.extras_mut().pattern = None;
                        out.push(seam.id);
                        keep.push(seam);
                    }
                    Some(_) => {}
                    None => keep.push((*c).clone()),
                }
            }
            beside.push((g, keep));
        }
        d.insert_beside(beside, true)?;
        sel.set(out.iter().copied());
        Ok(out)
    })?;
    Ok(ids_json(&ids))
}

/// Change the piece `id` with `f` (checked first on a copy), in one undo step labelled `label`.
fn change(s: &mut Session, id: NodeId, label: &str, cmd: &str, f: impl FnOnce(&mut Piece) -> Result<()>) -> Result<Value> {
    let mut piece = s.doc()?.doc.node(id).and_then(piece_on).cloned().ok_or_else(|| bad(cmd, "not a pattern piece"))?;
    f(&mut piece)?;
    s.edit(label, |d, _| {
        if let Some(n) = d.node_mut(id) {
            n.extras_mut().pattern = Some(Box::new(PatternTag::Piece(Box::new(piece))));
        }
        rebuild(d, id)
    })?;
    info_of(&s.doc()?.doc, id)
}

fn key_index(k: &str, cmd: &str) -> Result<usize> {
    k.trim().parse::<usize>().ok().filter(|i| *i < 100_000).ok_or_else(|| bad(cmd, "edges and corners are numbered from 0"))
}

fn point_of(v: &Value) -> Option<Pt> {
    let [x, y] = finite_numbers::<2>(v)?;
    Some(Pt::new(x, y))
}

fn set(s: &mut Session, p: &Value) -> Result<Value> {
    const CMD: &str = "piece.set";
    let id = one_piece(s, p, CMD)?;
    // The fold: a straight edge of the seam line.
    let fold = match p.get("fold") {
        None => None,
        Some(Value::Null) => Some(None),
        Some(Value::String(a)) if a == "auto" => {
            Some(Some(longest_straight(s, id).ok_or_else(|| bad(CMD, "the piece has no straight edge to cut on the fold"))?))
        }
        Some(v) => {
            let e = v.as_u64().map(|e| e as usize).ok_or_else(|| bad(CMD, "fold is an edge's number (0..), or null"))?;
            if e >= edge_count(s, id) {
                return Err(bad(CMD, format!("the piece has no edge {e}")));
            }
            if !straight(s, id, e) {
                return Err(bad(CMD, "the fold must be a straight edge"));
            }
            Some(Some(e))
        }
    };
    change(s, id, "Piece Options", CMD, |piece| {
        if let Some(f) = fold {
            piece.fold = f;
        }
        if let Some(n) = str_param(p, "name") {
            piece.name = pf_core::piece::check_name(n).map_err(|e| err(CMD, e))?;
        }
        if let Some(f) = str_param(p, "fabric") {
            piece.fabric = pf_core::piece::check_name(f).map_err(|e| err(CMD, e))?;
        }
        if let Some(v) = p.get("cut") {
            let cut = v.as_u64().and_then(|c| u32::try_from(c).ok()).ok_or_else(|| bad(CMD, "cut is a whole number, 1 to 999"))?;
            piece.set_cut(cut).map_err(|e| err(CMD, e))?;
        }
        if let Some(v) = p.get("mirror") {
            piece.mirror = v.as_bool().ok_or_else(|| bad(CMD, "mirror is true or false"))?;
        }
        if let Some(v) = p.get("allowance") {
            piece.allowance = allowance_param(v, CMD)?;
        }
        if let Some(edges) = p.get("edges") {
            let map = edges.as_object().ok_or_else(|| bad(CMD, "edges is {\"<edge>\": mm or null}"))?;
            for (k, v) in map {
                let e = key_index(k, CMD)?;
                if v.is_null() {
                    piece.edges.remove(&e);
                } else {
                    piece.edges.insert(e, allowance_param(v, CMD)?);
                }
            }
        }
        if let Some(corners) = p.get("corners") {
            let map = corners.as_object().ok_or_else(|| bad(CMD, "corners is {\"<corner>\": \"mitre\" or \"square\"}"))?;
            for (k, v) in map {
                let c = key_index(k, CMD)?;
                match v.as_str() {
                    Some("mitre" | "miter") => {
                        piece.corners.remove(&c);
                    }
                    Some("square") => {
                        piece.corners.insert(c, Corner::Square);
                    }
                    _ => return Err(bad(CMD, "a corner is \"mitre\" or \"square\"")),
                }
            }
        }
        match p.get("grainline") {
            None => {}
            Some(Value::String(a)) if a == "auto" => piece.grainline = None,
            Some(Value::Null) => piece.grainline = None,
            Some(v) => {
                let (Some(from), Some(to)) = (v.get("from").and_then(point_of), v.get("to").and_then(point_of)) else {
                    return Err(bad(CMD, "grainline is {from: [x, y], to: [x, y]} or \"auto\""));
                };
                if (to - from).len() < 1e-6 {
                    return Err(bad(CMD, "the grainline's ends are the same point"));
                }
                piece.grainline = Some(pf_core::Grainline { from, to });
            }
        }
        Ok(())
    })
}

fn add_notch(s: &mut Session, p: &Value) -> Result<Value> {
    const CMD: &str = "piece.addNotch";
    let id = one_piece(s, p, CMD)?;
    let edge =
        p.get("edge").and_then(Value::as_u64).and_then(|e| usize::try_from(e).ok()).ok_or_else(|| bad(CMD, "edge: the edge's number, from 0"))?;
    let at = match p.get("at") {
        None => 0.5,
        Some(v) => v.as_f64().ok_or_else(|| bad(CMD, "at: 0 (the edge's start) to 1 (its end)"))?,
    };
    let kind = match str_param(p, "kind") {
        None | Some("slit") => NotchKind::Slit,
        Some("double") => NotchKind::Double,
        Some("v") => NotchKind::V,
        Some(_) => return Err(bad(CMD, "kind: \"slit\", \"double\" or \"v\"")),
    };
    let edges = edge_count(s, id);
    if edge >= edges {
        return Err(bad(CMD, format!("the piece has edges 0 to {}", edges.saturating_sub(1))));
    }
    change(s, id, "Add Notch", CMD, |piece| piece.add_notch(Notch { edge, at, kind }).map_err(|e| err(CMD, e)))
}

fn remove_notch(s: &mut Session, p: &Value) -> Result<Value> {
    const CMD: &str = "piece.removeNotch";
    let id = one_piece(s, p, CMD)?;
    let index = p
        .get("index")
        .and_then(Value::as_u64)
        .and_then(|i| usize::try_from(i).ok())
        .ok_or_else(|| bad(CMD, "index: the notch's place in the list, from 0"))?;
    change(s, id, "Remove Notch", CMD, |piece| {
        if index >= piece.notches.len() {
            return Err(bad(CMD, "no notch with that index"));
        }
        piece.notches.remove(index);
        Ok(())
    })
}

fn seam_of(doc: &Document, id: NodeId) -> Option<Arc<Node>> {
    doc.node(id)?.children()?.iter().find(|c| mark_of(c) == Some(Mark::Seam)).cloned()
}

fn edge_count(s: &Session, id: NodeId) -> usize {
    let Ok(st) = s.doc() else { return 0 };
    seam_of(&st.doc, id)
        .and_then(|n| match &n.kind {
            NodeKind::Path { path, .. } => outline_of(path).map(|(_, e)| e),
            _ => None,
        })
        .unwrap_or(0)
}

/// The longest straight edge of piece `id`'s seam line.
fn longest_straight(s: &Session, id: NodeId) -> Option<usize> {
    let st = s.doc().ok()?;
    let seam = seam_of(&st.doc, id)?;
    let NodeKind::Path { path, .. } = &seam.kind else { return None };
    let sp = path.subpaths.iter().find(|sp| sp.closed)?;
    (0..sp.segment_count()).filter(|&e| sp.segment_is_line(e)).max_by(|&a, &b| {
        let len = |e: usize| {
            let g = sp.segment(e);
            (g.p3 - g.p0).hypot()
        };
        len(a).total_cmp(&len(b))
    })
}

/// Whether edge `e` of piece `id`'s seam line is straight.
fn straight(s: &Session, id: NodeId, e: usize) -> bool {
    let Ok(st) = s.doc() else { return false };
    seam_of(&st.doc, id).is_some_and(|n| match &n.kind {
        NodeKind::Path { path, .. } => path.subpaths.iter().find(|sp| sp.closed).is_some_and(|sp| e < sp.segment_count() && sp.segment_is_line(e)),
        _ => false,
    })
}

fn info_of(doc: &Document, id: NodeId) -> Result<Value> {
    let n = doc.node(id).ok_or_else(|| bad("piece.info", "no object with that id"))?;
    let piece = piece_on(n).ok_or_else(|| bad("piece.info", "not a pattern piece"))?;
    let seam = seam_of(doc, id);
    let mut v = serde_json::to_value(piece).map_err(|e| err("piece.info", e))?;
    let (mut edges, mut length, mut area) = (0, 0.0, 0.0);
    if let Some(NodeKind::Path { path, .. }) = seam.as_deref().map(|s| &s.kind)
        && let Some(sp) = path.subpaths.iter().find(|sp| sp.closed)
    {
        edges = sp.segment_count();
        length = pt_to_mm(PathData::single(sp.clone()).length());
        area = pt_to_mm(pt_to_mm(sp.area().abs())) / 100.0;
    }
    if let Some(o) = v.as_object_mut() {
        o.insert("id".into(), json!(id.0));
        o.insert("seam".into(), json!(seam.as_ref().map(|s| s.id.0)));
        o.insert("edgeCount".into(), json!(edges));
        o.insert("seamLength".into(), json!((length * 100.0).round() / 100.0));
        o.insert("area".into(), json!((area * 100.0).round() / 100.0));
        o.insert("problem".into(), json!(problem(seam.as_deref())));
    }
    Ok(v)
}

fn info(s: &mut Session, p: &Value) -> Result<Value> {
    let id = one_piece(s, p, "piece.info")?;
    info_of(&s.doc()?.doc, id)
}

fn list(s: &mut Session, _: &Value) -> Result<Value> {
    let doc = &s.doc()?.doc;
    let mut pieces = Vec::new();
    doc.walk(|n| {
        if let Some(pc) = piece_on(n) {
            pieces.push(json!({"id": n.id.0, "name": pc.name, "cut": pc.cut, "mirror": pc.mirror, "fabric": pc.fabric}));
        }
    });
    Ok(json!({ "pieces": pieces }))
}
