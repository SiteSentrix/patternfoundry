//! Measurements and drafting (PF-3): the document's measurement set and size chart, and path
//! points placed by formulas over them, so a draft follows the measurements.
//!
//! The measurements are personal data: they are kept in the document (the user's own file),
//! never logged, and left out of the copies of the document that exported PDF and SVG files
//! carry, unless the set says they may be shared ([`for_sharing`]).
//!
//! The user's own measurement sets can be kept on this computer, in a folder next to the
//! preferences (PF-3.4: `measure.saveSet`, `measure.sets`, `measure.loadSet`,
//! `measure.deleteSet`; [`crate::store`]), to load into any pattern.
//!
//! Lengths come and go in millimetres (a number; or a string with a unit, `"35 in"`).
use super::*;
use pf_core::draft::Draft;
use pf_core::measure::{Measurements, SizeChart};
use serde_json::{Map, Value, json};
use std::borrow::Cow;
use vectorcraft_doc::{Document, Node, NodeId, NodeKind};
use vectorcraft_geom::Point;

/// Longest size chart text accepted, in bytes.
const MAX_CHART_BYTES: usize = 1 << 20;

pub fn specs() -> Vec<CommandSpec> {
    vec![
        cmd!(
            "measure.set",
            "Set Measurements",
            [],
            None,
            "{name?, values?: {\"<name>\": mm (or a length with its unit, \"35 in\") | null (remove)}, formulas?: {\"<name>\": \"expression over the measurements, in mm\" | null}, share?: bool (let exported files' embedded copies keep the values; default false)} change the document's measurement set (names: letters, digits and _, starting with a letter) → measure.info",
            has_doc,
            set
        ),
        cmd!(
            query "measure.info",
            "Measurements",
            [],
            None,
            "{} → {name, values: {name: mm}, formulas: {name: text}, computed: {name: mm}, errors: {name: why}, size, share, chart: {name, columns, sizes} | null}",
            has_doc,
            info
        ),
        cmd!(
            "measure.clear",
            "Clear Measurements",
            [],
            None,
            "{} remove the document's measurements (drafted points stay where they are) → measure.info",
            has_doc,
            clear
        ),
        cmd!(
            "measure.importChart",
            "Import Size Chart",
            [],
            None,
            "{data: text (a header row: size, then measurement names; then a row per size; comma, semicolon or tab separated), name?, unit?: \"mm\" | \"cm\" (default) | \"in\"} the document's size chart → {name, columns, sizes}",
            has_doc,
            import_chart
        ),
        cmd!(
            "measure.useSize",
            "Use Size",
            [],
            None,
            "{size} copy a size of the size chart into the measurements (formulas stay) → measure.info",
            has_doc,
            use_size
        ),
        cmd!(
            query "measure.saveSet",
            "Save to My Measurements",
            [],
            None,
            "{name? (default: the set's own name)} keep the document's measurement set (values and formulas) on this computer, in the folder of saved sets next to the preferences, replacing a saved set of the same name; it never leaves the computer and is never shared → {name, sets}",
            has_doc,
            save_set
        ),
        cmd!(
            query "measure.sets",
            "My Measurements",
            [],
            None,
            "{} the measurement sets kept on this computer (measure.saveSet), by name, without their values → {sets: [{name, count (how many measurements), size}], store: bool (whether there is a folder to keep them in)}",
            has_doc,
            sets
        ),
        cmd!(
            "measure.loadSet",
            "Load Measurements",
            [],
            None,
            "{name} the saved set becomes the document's measurement set (blocks and point formulas follow it; not shared) → measure.info",
            has_doc,
            load_set
        ),
        cmd!(
            query "measure.deleteSet",
            "Delete Saved Measurements",
            [],
            None,
            "{name} delete a saved measurement set from this computer → {sets}",
            has_doc,
            delete_set
        ),
        cmd!(
            "draft.setPoint",
            "Set Point Formula",
            [],
            None,
            "{id? (a path, or a pattern piece's seam line; default: the selected one), anchor: 0.., x?: \"expression in mm\" | \"\" (clear), y?: …, origin?: [x, y] (document points the formulas measure from; default: the path's own origin, else the first artboard's top-left corner)} place the anchor point from the measurements → draft.info",
            has_doc,
            set_point
        ),
        cmd!(
            query "draft.info",
            "Draft Info",
            [],
            None,
            "{id?} → {id, origin: [x, y], points: [{anchor, x, y (formulas), at: [x, y] (where it is), errors}]}",
            has_doc,
            draft_info
        ),
        cmd!(
            "draft.clear",
            "Clear Point Formulas",
            [],
            None,
            "{id?} the path's points keep their places and lose their formulas → {id}",
            has_doc,
            draft_clear
        ),
    ]
}

/// The document as exported files carry it: the measured values left out unless the set may be
/// shared. Borrowed when there is nothing to leave out.
pub(crate) fn for_sharing(doc: &Document) -> Cow<'_, Document> {
    match &doc.measurements {
        Some(m) if !m.share && !m.values.is_empty() => {
            let mut d = doc.clone();
            d.measurements = Some(Box::new(m.for_sharing()));
            Cow::Owned(d)
        }
        _ => Cow::Borrowed(doc),
    }
}

fn err(cmd: &str, e: impl ToString) -> EngineError {
    bad(cmd, e.to_string())
}

/// A length param in millimetres: a number (mm) or a string with a unit.
fn mm_of(v: &Value) -> Option<f64> {
    match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => vectorcraft_doc::Unit::Millimeters.parse(s).map(pf_core::pt_to_mm),
        _ => None,
    }
    .filter(|v| v.is_finite())
}

fn info_of(doc: &Document) -> Value {
    let m = doc.measurements.as_deref().cloned().unwrap_or_default();
    let (mut computed, mut errors) = (Map::new(), Map::new());
    for (k, r) in m.computed() {
        match r {
            Ok(v) => {
                computed.insert(k, json!((v * 1000.0).round() / 1000.0));
            }
            Err(e) => {
                errors.insert(k, json!(e.to_string()));
            }
        }
    }
    let chart = doc.size_chart.as_deref().map(|c| json!({"name": c.name, "columns": c.columns, "sizes": c.size_names()}));
    json!({
        "name": m.name, "values": m.values, "formulas": m.formulas, "computed": computed, "errors": errors,
        "size": m.size, "share": m.share, "chart": chart,
    })
}

fn set(s: &mut Session, p: &Value) -> Result<Value> {
    const CMD: &str = "measure.set";
    let mut m = s.doc()?.doc.measurements.as_deref().cloned().unwrap_or_default();
    if let Some(n) = str_param(p, "name") {
        if n.chars().count() > 200 {
            return Err(bad(CMD, "a name takes at most 200 characters"));
        }
        m.name = n.trim().to_string();
    }
    if let Some(v) = p.get("values") {
        let map = v.as_object().ok_or_else(|| bad(CMD, "values is {\"<name>\": mm or null}"))?;
        for (k, v) in map {
            let mm = if v.is_null() {
                None
            } else {
                Some(mm_of(v).ok_or_else(|| bad(CMD, format!("{k}: a measurement is a number of mm, or a length with its unit")))?)
            };
            m.set_value(k, mm).map_err(|e| err(CMD, e))?;
        }
        // Measured by hand now: no longer just a size of the chart.
        if !map.is_empty() {
            m.size = None;
        }
    }
    if let Some(v) = p.get("formulas") {
        let map = v.as_object().ok_or_else(|| bad(CMD, "formulas is {\"<name>\": \"expression\" or null}"))?;
        for (k, v) in map {
            let text = match v {
                Value::Null => None,
                Value::String(t) => Some(t.as_str()),
                Value::Number(_) => return Err(bad(CMD, format!("{k}: give a formula as text (\"40\")"))),
                _ => return Err(bad(CMD, format!("{k}: a formula is text"))),
            };
            m.set_formula(k, text).map_err(|e| err(CMD, e))?;
        }
    }
    if let Some(v) = p.get("share") {
        m.share = v.as_bool().ok_or_else(|| bad(CMD, "share is true or false"))?;
    }
    s.edit("Set Measurements", |d, _| {
        d.measurements = Some(Box::new(m));
        Ok(())
    })?;
    Ok(info_of(&s.doc()?.doc))
}

fn info(s: &mut Session, _: &Value) -> Result<Value> {
    Ok(info_of(&s.doc()?.doc))
}

fn clear(s: &mut Session, _: &Value) -> Result<Value> {
    s.edit("Clear Measurements", |d, _| {
        d.measurements = None;
        Ok(())
    })?;
    Ok(info_of(&s.doc()?.doc))
}

fn no_store(cmd: &str) -> EngineError {
    bad(cmd, "there is no folder to keep measurement sets in here (the desktop app keeps them next to its preferences)")
}

fn set_list(s: &Session) -> Value {
    let sets: Vec<Value> =
        s.measurement_sets().iter().map(|m| json!({"name": m.name, "count": m.values.len() + m.formulas.len(), "size": m.size})).collect();
    json!(sets)
}

fn save_set(s: &mut Session, p: &Value) -> Result<Value> {
    const CMD: &str = "measure.saveSet";
    if !s.has_measurement_store() {
        return Err(no_store(CMD));
    }
    let mut m = s.doc()?.doc.measurements.as_deref().cloned().ok_or_else(|| bad(CMD, "the document has no measurements to save"))?;
    if m.values.is_empty() && m.formulas.is_empty() {
        return Err(bad(CMD, "the document has no measurements to save"));
    }
    if let Some(n) = str_param(p, "name") {
        m.name = n.trim().chars().take(200).collect();
    }
    if m.name.is_empty() {
        return Err(bad(CMD, "name: whose measurements they are (\"Me\")"));
    }
    if !s.store_measurement_set(&m) {
        return Err(bad(CMD, "the measurements could not be saved"));
    }
    Ok(json!({"name": m.name, "sets": set_list(s)}))
}

fn sets(s: &mut Session, _: &Value) -> Result<Value> {
    Ok(json!({"sets": set_list(s), "store": s.has_measurement_store()}))
}

fn named_set(s: &Session, p: &Value, cmd: &str) -> Result<Measurements> {
    if !s.has_measurement_store() {
        return Err(no_store(cmd));
    }
    let name = str_param(p, "name").map(str::trim).filter(|n| !n.is_empty()).ok_or_else(|| bad(cmd, "name: the saved set's name"))?;
    s.measurement_sets().into_iter().find(|m| m.name.eq_ignore_ascii_case(name)).ok_or_else(|| bad(cmd, format!("there is no saved set `{name}`")))
}

fn load_set(s: &mut Session, p: &Value) -> Result<Value> {
    const CMD: &str = "measure.loadSet";
    let m = named_set(s, p, CMD)?;
    s.edit("Load Measurements", |d, _| {
        d.measurements = Some(Box::new(m));
        Ok(())
    })?;
    Ok(info_of(&s.doc()?.doc))
}

fn delete_set(s: &mut Session, p: &Value) -> Result<Value> {
    const CMD: &str = "measure.deleteSet";
    let m = named_set(s, p, CMD)?;
    s.remove_measurement_set(&m.name);
    Ok(json!({"sets": set_list(s)}))
}

fn import_chart(s: &mut Session, p: &Value) -> Result<Value> {
    const CMD: &str = "measure.importChart";
    let data = str_param(p, "data").ok_or_else(|| bad(CMD, "data: the chart's text"))?;
    if data.len() > MAX_CHART_BYTES {
        return Err(bad(CMD, "the chart is larger than 1 MB"));
    }
    let unit = match str_param(p, "unit").unwrap_or("cm") {
        "mm" => 1.0,
        "cm" => 10.0,
        "in" => 25.4,
        _ => return Err(bad(CMD, "unit: \"mm\", \"cm\" or \"in\"")),
    };
    let chart = SizeChart::from_text(str_param(p, "name").unwrap_or(""), data, unit).map_err(|e| err(CMD, e))?;
    let out = json!({"name": chart.name, "columns": chart.columns, "sizes": chart.size_names()});
    s.edit("Import Size Chart", |d, _| {
        d.size_chart = Some(Box::new(chart));
        Ok(())
    })?;
    Ok(out)
}

fn use_size(s: &mut Session, p: &Value) -> Result<Value> {
    const CMD: &str = "measure.useSize";
    let size = str_param(p, "size").ok_or_else(|| bad(CMD, "size: one of the chart's sizes"))?;
    let doc = &s.doc()?.doc;
    let chart = doc.size_chart.as_deref().cloned().ok_or_else(|| bad(CMD, "the document has no size chart (measure.importChart)"))?;
    let mut m = doc.measurements.as_deref().cloned().unwrap_or_default();
    chart.apply(size, &mut m).map_err(|e| err(CMD, e))?;
    s.edit("Use Size", |d, _| {
        d.measurements = Some(Box::new(m));
        Ok(())
    })?;
    Ok(info_of(&s.doc()?.doc))
}

/// The path a draft command acts on: `id`, else the selected one; a pattern piece stands for its
/// seam line.
pub(crate) fn target_path(s: &Session, p: &Value, cmd: &str) -> Result<NodeId> {
    let st = s.doc()?;
    let id = match id_param(p, "id") {
        Some(id) => id,
        None => match st.selection.in_paint_order(&st.doc).as_slice() {
            [one] => *one,
            _ => return Err(bad(cmd, "select one path")),
        },
    };
    let n = st.doc.node(id).ok_or(EngineError::NoNode(id))?;
    if matches!(n.kind, NodeKind::Path { .. }) {
        return Ok(id);
    }
    // A pattern piece: its seam line.
    let seam = n.children().and_then(|ch| {
        ch.iter().find(|c| matches!(c.extras().pattern.as_deref(), Some(pf_core::PatternTag::Mark(pf_core::Mark::Seam)))).map(|c| c.id)
    });
    seam.ok_or_else(|| bad(cmd, "select a path or a pattern piece"))
}

fn anchors_of(n: &Node) -> usize {
    match &n.kind {
        NodeKind::Path { path, .. } => path.subpaths.first().map_or(0, |sp| sp.anchors.len()),
        _ => 0,
    }
}

/// Place the drafted points of every path with a draft from the measurements. Called inside
/// every edit (before pattern pieces redraw from their seam lines); returns at once in documents
/// without drafts.
pub(crate) fn refresh(doc: &mut Document) -> Result<()> {
    if !doc.drafts {
        return Ok(());
    }
    // Block parts are drawn whole from the measurements; then point formulas place their points.
    super::block::refresh(doc)?;
    let mut drafted = Vec::new();
    let mut blocks = false;
    doc.walk(|n| {
        if let Some(d) = n.extras().draft.as_deref() {
            drafted.push((n.id, d.clone()));
        }
        blocks |= n.extras().block.is_some();
    });
    doc.drafts = blocks || !drafted.is_empty();
    let mut env = doc.measurements.as_deref().map(Measurements::env).unwrap_or_default();
    for (id, draft) in drafted {
        let placed = draft.place(&mut env);
        let Some(n) = doc.node(id) else { continue };
        let NodeKind::Path { path, .. } = &n.kind else { continue };
        // Only write when a point moves, so unchanged drafts leave the document as it was.
        let Some(sp) = path.subpaths.first() else { continue };
        let moves: Vec<(usize, Point)> = placed
            .iter()
            .filter_map(|pl| {
                let a = sp.anchors.get(pl.anchor)?;
                let to = Point::new(pl.x.unwrap_or(a.p.x), pl.y.unwrap_or(a.p.y));
                ((to - a.p).hypot() > 1e-9).then_some((pl.anchor, to))
            })
            .collect();
        if moves.is_empty() {
            continue;
        }
        let path = super::path::path_mut(doc, id)?;
        if let Some(sp) = path.subpaths.first_mut() {
            for (i, to) in moves {
                if let Some(a) = sp.anchors.get_mut(i) {
                    let d = to - a.p;
                    a.p = to;
                    a.h_in += d;
                    a.h_out += d;
                }
            }
        }
    }
    Ok(())
}

fn draft_info_of(doc: &Document, id: NodeId) -> Result<Value> {
    let n = doc.node(id).ok_or(EngineError::NoNode(id))?;
    let draft = n.extras().draft.as_deref().cloned().unwrap_or_default();
    let mut env = doc.measurements.as_deref().map(Measurements::env).unwrap_or_default();
    let placed = draft.place(&mut env);
    let anchors: Vec<Point> = match &n.kind {
        NodeKind::Path { path, .. } => path.subpaths.first().map(|sp| sp.anchors.iter().map(|a| a.p).collect()).unwrap_or_default(),
        _ => vec![],
    };
    let points: Vec<Value> = draft
        .points
        .iter()
        .zip(&placed)
        .map(|(f, pl)| {
            let at = anchors.get(f.anchor).map(|p| json!([p.x, p.y]));
            json!({"anchor": f.anchor, "x": f.x, "y": f.y, "at": at, "errors": pl.errors})
        })
        .collect();
    Ok(json!({"id": id.0, "origin": [draft.origin.x, draft.origin.y], "points": points}))
}

fn set_point(s: &mut Session, p: &Value) -> Result<Value> {
    const CMD: &str = "draft.setPoint";
    let id = target_path(s, p, CMD)?;
    let doc = &s.doc()?.doc;
    let n = doc.node(id).ok_or(EngineError::NoNode(id))?;
    let anchor = p
        .get("anchor")
        .and_then(Value::as_u64)
        .and_then(|a| usize::try_from(a).ok())
        .ok_or_else(|| bad(CMD, "anchor: the point's number along the path, from 0"))?;
    let count = anchors_of(n);
    if anchor >= count {
        return Err(bad(CMD, format!("the path has points 0 to {}", count.saturating_sub(1))));
    }
    let mut draft = n.extras().draft.as_deref().cloned().unwrap_or_else(|| {
        let o = doc.artboards.first().map_or(Point::ZERO, |a| a.rect.origin());
        Draft { origin: pf_core::Pt::new(o.x, o.y), points: vec![] }
    });
    if let Some(o) = p.get("origin") {
        let [x, y] = finite_numbers::<2>(o).ok_or_else(|| bad(CMD, "origin: [x, y] in document points"))?;
        draft.origin = pf_core::Pt::new(x, y);
    }
    let axis = |k: &str| -> Result<Option<&str>> {
        match p.get(k) {
            None | Some(Value::Null) => Ok(None),
            Some(Value::String(t)) => Ok(Some(t.as_str())),
            Some(_) => Err(bad(CMD, format!("{k}: a formula is text, in mm (\"bust / 4\")"))),
        }
    };
    draft.set(anchor, axis("x")?, axis("y")?).map_err(|e| err(CMD, e))?;
    s.edit("Set Point Formula", |d, _| {
        if let Some(n) = d.node_mut(id) {
            n.extras_mut().draft = if draft.points.is_empty() { None } else { Some(Box::new(draft)) };
        }
        d.drafts = true;
        Ok(())
    })?;
    draft_info_of(&s.doc()?.doc, id)
}

fn draft_info(s: &mut Session, p: &Value) -> Result<Value> {
    let id = target_path(s, p, "draft.info")?;
    draft_info_of(&s.doc()?.doc, id)
}

fn draft_clear(s: &mut Session, p: &Value) -> Result<Value> {
    let id = target_path(s, p, "draft.clear")?;
    s.edit("Clear Point Formulas", |d, _| {
        if let Some(n) = d.node_mut(id) {
            n.extras_mut().draft = None;
        }
        Ok(())
    })?;
    Ok(json!({"id": id.0}))
}
