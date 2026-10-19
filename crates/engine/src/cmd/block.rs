//! Blocks (PF-4): `block.bodice` and `block.skirt` draft the close-fitting bodice block and the straight skirt block (back and front) from the
//! document's measurements ([`pf_core::block`]). Each part is a path that remembers its block, so
//! when a measurement changes, [`refresh`] draws it again (inside a pattern piece too: the piece
//! then redraws its cutting line).
use super::*;
use pf_core::block::{BlockKind, BlockRef, Part, Seg};
use pf_core::{Pt, mm_to_pt};
use serde_json::{Value, json};
use vectorcraft_doc::{Appearance, Document, Node, NodeId, NodeKind};
use vectorcraft_geom::{Anchor, PathData, Point, SubPath};

pub fn specs() -> Vec<CommandSpec> {
    vec![
        cmd!(
            "block.bodice",
            "Draft Bodice Block",
            ["Pattern"],
            None,
            "{origin?: [x, y] (document points: the back neck's top-left construction corner; default 20 mm inside the first artboard's top-left corner)} draft the close-fitting bodice block (back and front, half bodies) from the measurements: bust, waist, nape_to_waist, back_width, chest, shoulder, neck, armscye_depth (mm; measure.set). Each part follows the measurements when they change → {ids}",
            has_doc,
            bodice
        ),
        cmd!(
            "block.skirt",
            "Draft Skirt Block",
            ["Pattern"],
            None,
            "{origin?: [x, y] (document points: the centre back at the waist; default 20 mm inside the first artboard's top-left corner)} draft the basic straight skirt block (back and front, half bodies) from the measurements: waist, hip, waist_to_hip, skirt_length (mm; measure.set). Each part follows the measurements when they change → {ids}",
            has_doc,
            skirt
        ),
        cmd!(
            query "block.measurements",
            "Block Measurements",
            [],
            None,
            "{block?: \"bodice\" (default) | \"skirt\"} the measurements the block is drafted from, what each one is, and which the document's set lacks → {needed: [{name, what}], missing: [name]}",
            has_doc,
            measurements
        ),
    ]
}

/// A part's outline as a closed path in document points.
fn path_of(part: &Part, origin: Pt) -> PathData {
    let at = |p: Pt| Point::new(origin.x + mm_to_pt(p.x), origin.y + mm_to_pt(p.y));
    let mut anchors = vec![Anchor::corner(at(part.start))];
    for s in &part.segs {
        match *s {
            Seg::Line(p) => anchors.push(Anchor::corner(at(p))),
            Seg::Cubic(c1, c2, p) => {
                if let Some(last) = anchors.last_mut() {
                    last.h_out = at(c1);
                }
                let mut a = Anchor::corner(at(p));
                a.h_in = at(c2);
                anchors.push(a);
            }
        }
    }
    // The outline comes back to its start: the last point is the first.
    if anchors.len() > 1
        && let (Some(first), Some(last)) = (anchors.first().copied(), anchors.last().copied())
        && (first.p - last.p).hypot() < 1e-9
    {
        anchors.pop();
        if let Some(f) = anchors.first_mut() {
            f.h_in = last.h_in;
        }
    }
    PathData::single(SubPath::new(anchors, true))
}

fn env_of(doc: &Document) -> pf_core::expr::Env {
    doc.measurements.as_deref().map(pf_core::measure::Measurements::env).unwrap_or_default()
}

fn bodice(s: &mut Session, p: &Value) -> Result<Value> {
    make(s, p, BlockKind::Bodice, "block.bodice", "Draft Bodice Block")
}

fn skirt(s: &mut Session, p: &Value) -> Result<Value> {
    make(s, p, BlockKind::Skirt, "block.skirt", "Draft Skirt Block")
}

fn make(s: &mut Session, p: &Value, kind: BlockKind, cmd: &'static str, label: &'static str) -> Result<Value> {
    let st = s.doc()?;
    let origin = match p.get("origin") {
        Some(v) => {
            let [x, y] = finite_numbers::<2>(v).ok_or_else(|| bad(cmd, "origin: [x, y] in document points"))?;
            Pt::new(x, y)
        }
        None => {
            let o = st.doc.artboards.first().map_or(Point::ZERO, |a| a.rect.origin());
            Pt::new(o.x + mm_to_pt(20.0), o.y + mm_to_pt(20.0))
        }
    };
    let parts = pf_core::block::draft(kind, &mut env_of(&st.doc)).map_err(|e| bad(cmd, e.to_string()))?;
    let parent = st.insertion_parent();
    let ids = s.edit(label, |d, sel| {
        let mut ids = Vec::new();
        for (i, part) in parts.iter().enumerate() {
            let id = d.alloc_id();
            let mut n = Node::path(
                id,
                path_of(part, origin),
                Appearance::basic(vectorcraft_color::Paint::None, vectorcraft_color::Paint::solid(vectorcraft_color::Color::BLACK), 1.0),
            );
            n.name = Some(part.name.to_string());
            n.extras_mut().block = Some(Box::new(BlockRef { kind, part: i, origin }));
            d.insert(parent, usize::MAX, n)?;
            ids.push(id);
        }
        d.drafts = true;
        sel.set(ids.iter().copied());
        Ok(ids)
    })?;
    Ok(json!({ "ids": ids.iter().map(|i| i.0).collect::<Vec<_>>() }))
}

fn measurements(s: &mut Session, p: &Value) -> Result<Value> {
    let kind = match str_param(p, "block") {
        None | Some("bodice") => BlockKind::Bodice,
        Some("skirt") => BlockKind::Skirt,
        Some(_) => return Err(bad("block.measurements", "block: \"bodice\" or \"skirt\"")),
    };
    let list = pf_core::block::measurements(kind);
    let mut env = env_of(&s.doc()?.doc);
    let needed: Vec<Value> = list.iter().map(|(n, w)| json!({"name": n, "what": w})).collect();
    let missing: Vec<&str> = list.iter().filter(|(n, _)| !env.value(n).is_ok_and(|v| v > 0.0)).map(|(n, _)| *n).collect();
    Ok(json!({"needed": needed, "missing": missing}))
}

/// Draw every block part again from the measurements (only parts whose outline changed are
/// written). A block the measurements can't make keeps its last outline. Called inside every
/// edit from the drafting refresh.
pub(crate) fn refresh(doc: &mut Document) -> Result<()> {
    let mut parts: Vec<(NodeId, BlockRef)> = Vec::new();
    doc.walk(|n| {
        if let Some(b) = n.extras().block.as_deref() {
            parts.push((n.id, b.clone()));
        }
    });
    if parts.is_empty() {
        return Ok(());
    }
    let mut env = env_of(doc);
    let mut drafted: Vec<(BlockKind, Option<Vec<Part>>)> = Vec::new();
    for (id, b) in parts {
        let made = match drafted.iter().find(|(k, _)| *k == b.kind) {
            Some((_, m)) => m.clone(),
            None => {
                let m = pf_core::block::draft(b.kind, &mut env).ok();
                drafted.push((b.kind, m.clone()));
                m
            }
        };
        let Some(part) = made.as_ref().and_then(|m| m.get(b.part)) else { continue };
        let new = path_of(part, b.origin);
        let same = doc.node(id).is_some_and(|n| matches!(&n.kind, NodeKind::Path { path, .. } if *path == new));
        if !same
            && let Some(n) = doc.node_mut(id)
            && let NodeKind::Path { path, live, .. } = &mut n.kind
        {
            *path = new;
            *live = None;
        }
    }
    Ok(())
}
