//! Blocks (PF-4): `block.bodice`, `block.skirt`, `block.sleeve` and `block.trousers` draft the
//! close-fitting bodice block, the straight skirt block (back and front), the one-piece sleeve
//! block and the trouser block (front and back) from the document's measurements ([`pf_core::block`]). Each part is a path that remembers its block, so
//! when a measurement changes, [`refresh`] draws it again (inside a pattern piece too: the piece
//! then redraws its cutting line).
use super::*;
use pf_core::block::{BlockKind, BlockRef, Options, Part, Seg};
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
            "{origin?: [x, y] (document points: the back neck's top-left construction corner; default 20 mm inside the first artboard's top-left corner), options?: {name: mm} (the block's ease and darts, see block.options)} draft the close-fitting bodice block (back and front, half bodies) from the measurements: bust, waist, nape_to_waist, back_width, chest, shoulder, neck, armscye_depth (mm; measure.set). Each part follows the measurements when they change → {ids}",
            has_doc,
            bodice
        ),
        cmd!(
            "block.skirt",
            "Draft Skirt Block",
            ["Pattern"],
            None,
            "{origin?: [x, y] (document points: the centre back at the waist; default 20 mm inside the first artboard's top-left corner), options?: {name: mm} (the block's ease and darts, see block.options)} draft the basic straight skirt block (back and front, half bodies) from the measurements: waist, hip, waist_to_hip, skirt_length (mm; measure.set). Each part follows the measurements when they change → {ids}",
            has_doc,
            skirt
        ),
        cmd!(
            "block.sleeve",
            "Draft Sleeve Block",
            ["Pattern"],
            None,
            "{origin?: [x, y] (document points: the shoulder point at the top of the crown; default: the sleeve 20 mm inside the first artboard's top-left corner), options?: {name: mm} (the block's ease and darts, see block.options)} draft the one-piece straight sleeve block from the measurements: top_arm, sleeve_length (mm; measure.set), and sleeve_crown if the set has it (the crown's length: the bodice's armhole plus the ease to set it in, 3 to 4.5 cm; a formula such as \"armhole + 30\"), which sets the crown depth instead of the top arm table; without it, where the document has a bodice block, the crown is that bodice's armhole plus the crown_ease option (44.45 mm, the middle of a classic sleeve's ease). It follows the measurements and the bodice when they change → {ids, crown (mm: the crown's length, underarm to underarm)}",
            has_doc,
            sleeve
        ),
        cmd!(
            "block.trousers",
            "Draft Trouser Block",
            ["Pattern"],
            None,
            "{origin?: [x, y] (document points: the frame's top-left corner, at the centre front's waist line; default: the block 20 mm inside the first artboard's top-left corner), options?: {name: mm} (the block's ease and darts, see block.options)} draft the basic trouser block (front and back, side by side) from the measurements: waist, hip, waist_to_hip, crotch_depth, waist_to_knee, waist_to_ankle, trouser_hem (mm; measure.set). Each part follows the measurements when they change → {ids}",
            has_doc,
            trousers
        ),
        cmd!(
            "block.mensTrousers",
            "Draft Men's Trouser Block",
            ["Pattern"],
            None,
            "{origin?: [x, y] (document points: the block's top-left; default: the block 20 mm inside the first artboard's top-left corner), options?: {name: mm} (its waistband, an easy fit, the back dart, see block.options)} draft the men's trouser block (PF-4.12; front and back, side by side, each one leg with the fork on the left) from the measurements: waist, hip (round the seat), crotch_depth (the body rise), waist_to_ankle, trouser_hem (mm; measure.set). Each part follows the measurements when they change → {ids}",
            has_doc,
            mens_trousers
        ),
        cmd!(
            query "block.options",
            "Block Options Info",
            [],
            None,
            "{id? (default: the selected block part)} the drafted block's options, its ease and darts → {id, block, part, options: [{name, what, value (mm, null: the default), default (mm, null: worked out from the measurements), min, max}]}",
            has_doc,
            options
        ),
        cmd!(
            "block.setOptions",
            "Block Options",
            [],
            None,
            "{id? (default: the selected block part), options: {name: mm | null (back to the block's default)}} change a drafted block's ease and darts (block.options lists them); every part of the block (drafted together, where it was drafted) is drawn again, and pieces made from it follow → {id, block, parts, options}",
            has_doc,
            set_options
        ),
        cmd!(
            "block.removeStep",
            "Remove Design Step",
            [],
            None,
            "{id? (default: the selected block part), index (the step's place in block.options' steps, from 0)} remove a design step from a drafted block part, and the steps after it (they were made on the outline it made): the part is drafted again and the steps before it made again in order. A split can't be removed (it made two panels: delete the one not wanted) → {id, steps (those left)}",
            has_doc,
            remove_step
        ),
        cmd!(
            query "block.measurements",
            "Block Measurements",
            [],
            None,
            "{block?: \"bodice\" (default) | \"skirt\" | \"sleeve\" | \"trousers\"} the measurements the block is drafted from, what each one is, and which the document's set lacks → {needed: [{name, what}], missing: [name]}",
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

/// The armhole of the document's bodice block (mm), drafted with its options: the sleeve block is
/// set into it (PF-4.9). None without a bodice block, or one the measurements can't make.
fn bodice_armhole(doc: &Document, env: &mut pf_core::expr::Env) -> Option<f64> {
    let mut options = None;
    doc.walk(|n| {
        if options.is_none()
            && let Some(b) = n.extras().block.as_deref()
            && b.kind == BlockKind::Bodice
        {
            options = Some(b.options.clone());
        }
    });
    pf_core::block::bodice_armhole(env, &options?).ok()
}

/// A block's parts as `doc` drafts them: the sleeve set into the document's bodice armhole.
fn draft_in(
    doc: &Document,
    kind: BlockKind,
    env: &mut pf_core::expr::Env,
    options: &Options,
) -> std::result::Result<Vec<Part>, pf_core::block::BlockError> {
    match kind {
        BlockKind::Sleeve => {
            let armhole = bodice_armhole(doc, env);
            pf_core::block::sleeve_for(env, options, armhole)
        }
        _ => pf_core::block::draft(kind, env, options),
    }
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

fn trousers(s: &mut Session, p: &Value) -> Result<Value> {
    make(s, p, BlockKind::Trousers, "block.trousers", "Draft Trouser Block")
}

fn mens_trousers(s: &mut Session, p: &Value) -> Result<Value> {
    make(s, p, BlockKind::MensTrousers, "block.mensTrousers", "Draft Men's Trouser Block")
}

fn sleeve(s: &mut Session, p: &Value) -> Result<Value> {
    let mut r = make(s, p, BlockKind::Sleeve, "block.sleeve", "Draft Sleeve Block")?;
    let options = options_param(p, BlockKind::Sleeve, "block.sleeve")?;
    let doc = &s.doc()?.doc;
    let parts = draft_in(doc, BlockKind::Sleeve, &mut env_of(doc), &options).map_err(|e| bad("block.sleeve", e.to_string()))?;
    let crown = parts.first().map_or(0.0, pf_core::block::sleeve_crown);
    r["crown"] = json!(crown);
    Ok(r)
}

fn make(s: &mut Session, p: &Value, kind: BlockKind, cmd: &'static str, label: &'static str) -> Result<Value> {
    let st = s.doc()?;
    let origin = match p.get("origin") {
        Some(v) => {
            let [x, y] = finite_numbers::<2>(v).ok_or_else(|| bad(cmd, "origin: [x, y] in document points"))?;
            Some(Pt::new(x, y))
        }
        None => None,
    };
    let options = options_param(p, kind, cmd)?;
    let parts = draft_in(&st.doc, kind, &mut env_of(&st.doc), &options).map_err(|e| bad(cmd, e.to_string()))?;
    // By default the drawing (not its origin) starts 20 mm inside the first artboard's corner.
    let origin = origin.unwrap_or_else(|| {
        let o = st.doc.artboards.first().map_or(Point::ZERO, |a| a.rect.origin());
        let (left, top) = parts.iter().flat_map(Part::corners).fold((0.0_f64, 0.0_f64), |(l, t), q| (l.min(q.x), t.min(q.y)));
        Pt::new(o.x + mm_to_pt(20.0 - left), o.y + mm_to_pt(20.0 - top))
    });
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
            n.extras_mut().block = Some(Box::new(BlockRef { kind, part: i, origin, options: options.clone(), steps: Vec::new() }));
            d.insert(parent, usize::MAX, n)?;
            ids.push(id);
        }
        d.drafts = true;
        sel.set(ids.iter().copied());
        Ok(ids)
    })?;
    Ok(json!({ "ids": ids.iter().map(|i| i.0).collect::<Vec<_>>() }))
}

/// The `options` param: `{name: mm}` options of `kind`, checked.
fn options_param(p: &Value, kind: BlockKind, cmd: &str) -> Result<Options> {
    let Some(v) = p.get("options").filter(|v| !v.is_null()) else { return Ok(Options::new()) };
    let o = v.as_object().ok_or_else(|| bad(cmd, "options: {name: mm}"))?;
    let mut out = Options::new();
    for (k, v) in o {
        let mm = super::piece::mm_param(v).ok_or_else(|| bad(cmd, format!("{k}: a number of mm, or a length with its unit")))?;
        out.insert(k.clone(), mm);
    }
    pf_core::block::check_options(kind, &out).map_err(|e| bad(cmd, e.to_string()))?;
    Ok(out)
}

/// The drafted block part `id` (default: the one selected): its id and reference.
fn block_part(s: &Session, p: &Value, cmd: &str) -> Result<(NodeId, BlockRef)> {
    let st = s.doc()?;
    let id = match p.get("id") {
        Some(v) => NodeId(v.as_u64().ok_or_else(|| bad(cmd, "id: a block part's id"))?),
        None => {
            // A selected block part, or the seam line of a selected piece made from one.
            let drafted = |id: NodeId| -> Option<NodeId> {
                let n = st.doc.node(id)?;
                if n.extras().block.is_some() {
                    return Some(id);
                }
                n.children()?.iter().find(|c| c.extras().block.is_some()).map(|c| c.id)
            };
            st.selection
                .in_paint_order(&st.doc)
                .into_iter()
                .find_map(drafted)
                .ok_or_else(|| bad(cmd, "select a drafted block part (Pattern › Draft … Block)"))?
        }
    };
    let b = st.doc.node(id).and_then(|n| n.extras().block.as_deref()).cloned().ok_or_else(|| bad(cmd, "not a drafted block part"))?;
    Ok((id, b))
}

/// The options of a drafted block and its parts.
fn options_info(kind: BlockKind, set: &Options) -> Value {
    let list: Vec<Value> = pf_core::block::options(kind)
        .iter()
        .map(|o| json!({"name": o.name, "what": o.what, "value": set.get(o.name), "default": o.default, "min": o.min, "max": o.max}))
        .collect();
    json!(list)
}

fn kind_name(kind: BlockKind) -> &'static str {
    match kind {
        BlockKind::Bodice => "bodice",
        BlockKind::Skirt => "skirt",
        BlockKind::Sleeve => "sleeve",
        BlockKind::Trousers => "trousers",
        BlockKind::MensTrousers => "mensTrousers",
    }
}

fn options(s: &mut Session, p: &Value) -> Result<Value> {
    let (id, b) = block_part(s, p, "block.options")?;
    Ok(json!({"id": id.0, "block": kind_name(b.kind), "part": b.part, "options": options_info(b.kind, &b.options), "steps": b.steps}))
}

fn set_options(s: &mut Session, p: &Value) -> Result<Value> {
    const CMD: &str = "block.setOptions";
    let (id, b) = block_part(s, p, CMD)?;
    let given = p.get("options").and_then(Value::as_object).ok_or_else(|| bad(CMD, "options: {name: mm | null (back to the default)}"))?;
    let mut next = b.options.clone();
    for (k, v) in given {
        if v.is_null() {
            next.remove(k);
        } else {
            let mm = super::piece::mm_param(v).ok_or_else(|| bad(CMD, format!("{k}: a number of mm, or a length with its unit")))?;
            next.insert(k.clone(), mm);
        }
    }
    pf_core::block::check_options(b.kind, &next).map_err(|e| bad(CMD, e.to_string()))?;
    // The block must still draft with them.
    let doc = &s.doc()?.doc;
    draft_in(doc, b.kind, &mut env_of(doc), &next).map_err(|e| bad(CMD, e.to_string()))?;
    // Every part of the block: the same kind, drafted at the same origin with the same options.
    let mut ids = Vec::new();
    s.doc()?.doc.walk(|n| {
        if let Some(o) = n.extras().block.as_deref()
            && o.kind == b.kind
            && o.origin == b.origin
            && o.options == b.options
        {
            ids.push(n.id);
        }
    });
    if !ids.contains(&id) {
        ids.push(id);
    }
    s.edit("Block Options", |d, _| {
        for i in &ids {
            if let Some(n) = d.node_mut(*i)
                && let Some(r) = n.extras_mut().block.as_deref_mut()
            {
                r.options = next.clone();
            }
        }
        refresh(d)
    })?;
    Ok(json!({"id": id.0, "block": kind_name(b.kind), "parts": ids.iter().map(|i| i.0).collect::<Vec<_>>(), "options": options_info(b.kind, &next)}))
}

fn remove_step(s: &mut Session, p: &Value) -> Result<Value> {
    const CMD: &str = "block.removeStep";
    let (id, b) = block_part(s, p, CMD)?;
    let index = p.get("index").and_then(Value::as_u64).ok_or_else(|| bad(CMD, "index: the step's place, from 0"))? as usize;
    if index >= b.steps.len() {
        return Err(bad(CMD, format!("index: the part has {} design steps", b.steps.len())));
    }
    // A later step's anchors are on the outline this one made, so they go with it.
    if b.steps[index..].iter().any(|st| matches!(st, pf_core::design::DesignStep::Split { .. })) {
        return Err(bad(CMD, "a split made two panels: delete the panel you don't want instead"));
    }
    let steps = b.steps[..index].to_vec();
    s.edit("Remove Design Step", |d, _| {
        if let Some(n) = d.node_mut(id)
            && let Some(r) = n.extras_mut().block.as_deref_mut()
        {
            r.steps = steps.clone();
        }
        refresh(d)
    })?;
    Ok(json!({"id": id.0, "steps": steps}))
}

fn measurements(s: &mut Session, p: &Value) -> Result<Value> {
    let kind = match str_param(p, "block") {
        None | Some("bodice") => BlockKind::Bodice,
        Some("skirt") => BlockKind::Skirt,
        Some("sleeve") => BlockKind::Sleeve,
        Some("trousers") => BlockKind::Trousers,
        Some("mensTrousers") => BlockKind::MensTrousers,
        Some(_) => return Err(bad("block.measurements", "block: \"bodice\", \"skirt\", \"sleeve\", \"trousers\" or \"mensTrousers\"")),
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
    // Each block drafted once for each set of options its parts have.
    let mut drafted: Vec<(BlockKind, Options, Option<Vec<Part>>)> = Vec::new();
    for (id, b) in parts {
        let made = match drafted.iter().find(|(k, o, _)| *k == b.kind && *o == b.options) {
            Some((_, _, m)) => m.clone(),
            None => {
                let m = draft_in(doc, b.kind, &mut env, &b.options).ok();
                drafted.push((b.kind, b.options.clone(), m.clone()));
                m
            }
        };
        let Some(part) = made.as_ref().and_then(|m| m.get(b.part)) else { continue };
        let drafted = path_of(part, b.origin);
        // The design steps made on it, again (PF-5.20); one that no longer works keeps the last
        // outline.
        let new = if b.steps.is_empty() {
            drafted
        } else {
            match super::design::replay(&drafted, &b.steps) {
                Some(p) => p,
                None => continue,
            }
        };
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
