//! Grading (PF-6): a pattern in every size of its size chart.
//!
//! Grading by redrafting: grading "allows the one pattern to be made, then sized up and down"
//! (https://uphance.com/blog/pattern-grading-a-beginners-guide/). A PatternFoundry draft already
//! follows its measurements (PF-3), so each size is the same draft made again from that size's
//! measurements: blocks, point formulas and the design steps on them, with every piece's
//! allowance, darts and notches. The sizes are drawn over each other as a nest, the way graders
//! check the progression from size to size, on a locked layer "Graded Nest" with a sublayer per
//! size (so a PDF with layers, or a print, can show one size or several).
//!
//! The nest is drawn from the size chart's measurements, not the user's own: it stores outlines
//! only, never measurements.

use super::*;
use pf_core::grade::{GradeRule, GradeSizes, Rule};
use pf_core::measure::Measurements;
use pf_core::{Mark, pt_to_mm};
use serde_json::{Value, json};
use std::sync::Arc;
use vectorcraft_color::{Color, Paint};
use vectorcraft_doc::{Appearance, Document, LayerColor, Node, NodeId, NodeKind};
use vectorcraft_geom::PathData;

/// The nest layer's name; making the nest again replaces the layer of this name.
pub(crate) const NEST: &str = "Graded Nest";

/// How far from a seam line's point a point picked on the canvas may be and still name it, mm.
const PICK_MM: f64 = 10.0;

/// Most sizes in one nest.
const MAX_SIZES: usize = 60;

/// The sizes' line colours, in turn (distinct on white and in grey).
const COLORS: [(u8, u8, u8); 8] =
    [(0, 114, 178), (213, 94, 0), (0, 158, 115), (204, 121, 167), (86, 180, 233), (230, 159, 0), (120, 94, 240), (0, 0, 0)];

pub fn specs() -> Vec<CommandSpec> {
    vec![
        cmd!(
            "grade.nest",
            "Grade Nest",
            [],
            None,
            "{sizes?: [size…] (default every size of the size chart, in its order), line?: \"cut\" (default) | \"seam\", align?: \"drafted\" (default: where each size is drafted) | \"topLeft\" | \"topRight\" | \"bottomLeft\" | \"bottomRight\" | \"center\" (each piece's sizes lined up on that point of their bounds, at the document's own piece) | {\"<piece name>\": …, \"*\": … (the rest)}} draft the pattern again in each size from the size chart's measurements and draw the sizes over each other on the locked layer \"Graded Nest\", a sublayer per size (made again when it exists; the document's own measurements and pieces are not changed) → {layer, sizes: [{size, layer, pieces: [{name, widthMm, heightMm, lengthMm}]}], warnings}",
            has_doc,
            nest
        ),
        cmd!(
            "grade.setSizes",
            "Set Size Range",
            [],
            None,
            "{sizes: [size…] (smallest first) | null (remove), base: one of them (the size the pattern is drawn in)} the sizes the pieces' grade rules grade into (grade.nest grades into them, from the size chart's measurements too where it has the size) → {sizes, base} | {sizes: null}",
            has_doc,
            set_sizes
        ),
        cmd!(
            "grade.setRule",
            "Set Grade Rule",
            [],
            None,
            "{id? (a pattern piece or its part; default: the selected piece), anchor: 0.. (a point of the seam line) | point: [x, y] (instead: document points, as a click on the canvas: the seam line's point within 10 mm), step?: [dx, dy] (mm the point moves each size up from the base, the opposite way each size down), across?: mm, down?: mm (the step as a dialog gives it: across to the right, down) | sizes?: {\"<size>\": [dx, dy] (mm from the base)} | clear?: true} x to the right, y down. Points between ruled points move in proportion along the seam line → grade.info",
            has_doc,
            set_rule
        ),
        cmd!(
            query "grade.info",
            "Grade Info",
            [],
            None,
            "{id?} the piece's grade rules and how far each seam line point moves in each size → {id, sizes, base, rules: [{anchor, step: [dx, dy]} | {anchor, sizes: {size: [dx, dy]}}], moves: {size: [[dx, dy] per point, mm]}}",
            has_doc,
            info
        ),
        cmd!(
            "grade.copyRules",
            "Copy Grade Rules",
            [],
            None,
            "{from: a pattern piece, to?: [pieces] (default: the selected pieces but `from`), mirror?: \"none\" (default) | \"x\" | \"y\" | \"xy\"} give each piece `from`'s grade rules, each on the point nearest the same place in the piece's bounds; mirror x for the opposite half (a front from a back drawn the other way round): places mirrored left to right and x moves turned the other way. The pieces' own rules are replaced → {pieces: [grade.info…]}",
            has_doc,
            copy_rules
        ),
        cmd!("grade.clear", "Clear Graded Nest", [], None, "{} remove the \"Graded Nest\" layer → {removed}", has_doc, clear),
    ]
}

/// Which point of each piece the sizes are lined up on.
#[derive(Clone, Copy, PartialEq)]
enum Align {
    /// Where each size is drafted (from the draft's origin).
    Drafted,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
    Center,
}

impl Align {
    fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "drafted" => Align::Drafted,
            "topLeft" => Align::TopLeft,
            "topRight" => Align::TopRight,
            "bottomLeft" => Align::BottomLeft,
            "bottomRight" => Align::BottomRight,
            "center" => Align::Center,
            _ => return None,
        })
    }

    /// The point of `r` this lines up on (`None`: as drafted).
    fn at(self, r: vectorcraft_geom::Rect) -> Option<vectorcraft_geom::Point> {
        use vectorcraft_geom::Point;
        Some(match self {
            Align::Drafted => return None,
            Align::TopLeft => Point::new(r.x0, r.y0),
            Align::TopRight => Point::new(r.x1, r.y0),
            Align::BottomLeft => Point::new(r.x0, r.y1),
            Align::BottomRight => Point::new(r.x1, r.y1),
            Align::Center => r.center(),
        })
    }
}

const ALIGNS: &str = "\"drafted\", \"topLeft\", \"topRight\", \"bottomLeft\", \"bottomRight\" or \"center\"";

/// The `align` param: one alignment for every piece, or one per piece name (`"*"` for the rest).
fn align_of(p: &Value, name: &str, cmd: &str) -> Result<Align> {
    let pick = |v: &Value| v.as_str().and_then(Align::parse).ok_or_else(|| bad(cmd, format!("align: {ALIGNS}, or {{\"<piece>\": …, \"*\": …}}")));
    match p.get("align") {
        None | Some(Value::Null) => Ok(Align::Drafted),
        Some(Value::Object(m)) => match m.iter().find(|(k, _)| k.trim().eq_ignore_ascii_case(name)).or_else(|| m.iter().find(|(k, _)| *k == "*")) {
            Some((_, v)) => pick(v),
            None => Ok(Align::Drafted),
        },
        Some(v) => pick(v),
    }
}

/// One piece of one size: its name and its line.
struct Drawn {
    name: String,
    path: PathData,
}

/// The seam line of the piece `n` and its points.
fn seam_of(n: &Node) -> Option<(NodeId, Vec<pf_core::Pt>)> {
    let seam = n.children()?.iter().find(|c| super::piece::mark_of(c) == Some(Mark::Seam))?;
    let NodeKind::Path { path, .. } = &seam.kind else { return None };
    let pts = path.subpaths.first()?.anchors.iter().map(|a| pf_core::Pt::new(a.p.x, a.p.y)).collect();
    Some((seam.id, pts))
}

/// Move every graded piece's seam line points by their grade rules for `size`.
fn apply_rules(d: &mut Document, sizes: &GradeSizes, size: &str) -> Result<()> {
    let mut graded = Vec::new();
    d.walk(|n| {
        if let Some(piece) = super::piece::piece_on(n)
            && !piece.grade.is_empty()
            && let Some((seam, pts)) = seam_of(n)
        {
            graded.push((seam, pf_core::grade::moves(&pts, &piece.grade, sizes, size)));
        }
    });
    for (seam, moves) in graded {
        let path = super::path::path_mut(d, seam)?;
        if let Some(sp) = path.subpaths.first_mut() {
            for (a, m) in sp.anchors.iter_mut().zip(moves) {
                let v = vectorcraft_geom::Point::new(pf_core::mm_to_pt(m[0]), pf_core::mm_to_pt(m[1])).to_vec2();
                a.p += v;
                a.h_in += v;
                a.h_out += v;
            }
        }
    }
    Ok(())
}

/// `doc` drafted from `m`, then graded to `rules`' size, its pieces drawn again.
fn graded(doc: &Document, m: Measurements, rules: Option<(&GradeSizes, &str)>) -> Result<Document> {
    let mut d = doc.clone();
    d.measurements = Some(Box::new(m));
    // As an edit does: drafted points follow the measurements, then pieces their seam lines;
    // the grade rules move the drafted points.
    super::measure::refresh(&mut d)?;
    if let Some((sizes, size)) = rules {
        apply_rules(&mut d, sizes, size)?;
    }
    super::piece::refresh(doc, &mut d)?;
    Ok(d)
}

/// `doc` with its pattern pieces in size `size` of the size range or the size chart, as the
/// graded nest draws that size; the document's own measurements kept.
pub(crate) fn in_size(doc: &Document, size: &str, cmd: &str) -> Result<Document> {
    let chart = doc.size_chart.as_deref();
    let range = doc.grade_sizes.as_deref();
    let in_range = range.and_then(|r| r.find(size)).map(String::from);
    let in_chart = chart.and_then(|c| c.size_names().into_iter().find(|n| n.eq_ignore_ascii_case(size.trim())).map(String::from));
    let Some(name) = in_range.clone().or(in_chart.clone()) else {
        return Err(bad(cmd, format!("there is no size `{size}` (the size chart's or the size range's)")));
    };
    let own = doc.measurements.as_deref().cloned().unwrap_or_default();
    let mut m = own.clone();
    if let (Some(c), Some(_)) = (chart, &in_chart) {
        c.apply(&name, &mut m).map_err(|e| bad(cmd, e.to_string()))?;
    }
    let mut d = graded(doc, m, range.zip(in_range.as_deref()))?;
    // The drawn size's measurements were only for drafting: the document keeps its own.
    d.measurements = doc.measurements.clone();
    Ok(d)
}

/// The pieces' lines of `doc` drafted from `m`, then graded to `rules`' size.
fn redraft(doc: &Document, m: Measurements, rules: Option<(&GradeSizes, &str)>, line: Mark, cmd: &str) -> Result<Vec<Drawn>> {
    let d = graded(doc, m, rules)?;
    let mut out = Vec::new();
    d.walk(|n| {
        let Some(piece) = super::piece::piece_on(n) else { return };
        let Some(path) = n.children().into_iter().flatten().find(|c| super::piece::mark_of(c) == Some(line)).and_then(|c| match &c.kind {
            NodeKind::Path { path, .. } => Some(path.clone()),
            _ => None,
        }) else {
            return;
        };
        // An unnamed piece goes by its seam line's name (a block part's).
        let seam = n.children().into_iter().flatten().find(|c| super::piece::mark_of(c) == Some(Mark::Seam)).and_then(|c| c.name.as_deref());
        let name = [Some(piece.name.as_str()), seam].into_iter().flatten().map(str::trim).find(|s| !s.is_empty());
        out.push(Drawn { name: name.unwrap_or("Piece").to_string(), path });
    });
    if out.is_empty() {
        return Err(bad(cmd, "no pattern pieces to grade: make the outlines pattern pieces first (Pattern › Make Pattern Piece)"));
    }
    Ok(out)
}

fn nest(s: &mut Session, p: &Value) -> Result<Value> {
    const CMD: &str = "grade.nest";
    let line = match str_param(p, "line").unwrap_or("cut") {
        "cut" => Mark::Cut,
        "seam" => Mark::Seam,
        _ => return Err(bad(CMD, "line: \"cut\" or \"seam\"")),
    };
    let doc = s.doc()?.doc.clone();
    let chart = doc.size_chart.as_deref();
    let range = doc.grade_sizes.as_deref();
    // The sizes: the grade rules' size range, else the size chart's.
    let all: Vec<String> = match (range, chart) {
        (Some(r), _) => r.sizes.clone(),
        (None, Some(c)) => c.size_names().into_iter().map(String::from).collect(),
        (None, None) => {
            return Err(bad(
                CMD,
                "the document has no sizes: import a size chart (Measurements › Import Size Chart) or name the sizes (grade.setSizes)",
            ));
        }
    };
    let sizes: Vec<String> = match p.get("sizes") {
        None | Some(Value::Null) => all,
        Some(Value::Array(v)) => {
            let mut out = Vec::new();
            for s in v {
                let s = s.as_str().ok_or_else(|| bad(CMD, "sizes: the sizes' names"))?;
                let name = all.iter().find(|n| n.eq_ignore_ascii_case(s.trim())).ok_or_else(|| bad(CMD, format!("there is no size `{s}`")))?;
                out.push(name.clone());
            }
            out
        }
        Some(_) => return Err(bad(CMD, "sizes: the sizes' names")),
    };
    if sizes.is_empty() {
        return Err(bad(CMD, "sizes: name one size at least"));
    }
    if sizes.len() > MAX_SIZES {
        return Err(bad(CMD, "too many sizes at once"));
    }
    // The document without its old nest: the base the sizes are drafted from.
    let mut base = (*doc).clone();
    base.layers.retain(|l| l.name.as_deref() != Some(NEST));
    let own = base.measurements.as_deref().cloned().unwrap_or_default();
    let as_is = redraft(&base, own.clone(), None, line, CMD)?;
    let mut drawn = Vec::new();
    let mut warnings = Vec::new();
    for size in &sizes {
        // The chart's measurements for the size, when it has the size.
        let mut m = own.clone();
        let charted = chart.is_some_and(|c| c.size_names().iter().any(|n| n == size));
        if let Some(c) = chart.filter(|_| charted) {
            c.apply(size, &mut m).map_err(|e| bad(CMD, e.to_string()))?;
        }
        // The size the document is drawn in: the rules' base size, else the chart size in use.
        let current = match range {
            Some(r) => r.base == *size && (!charted || m.size == own.size),
            None => own.size.is_some() && m.size == own.size,
        };
        let mut pieces = redraft(&base, m, range.map(|r| (r, size.as_str())), line, CMD)?;
        // Lined up on the document's own pieces, at the point asked.
        for (pc, own) in pieces.iter_mut().zip(&as_is) {
            let (Some(b), Some(o)) = (pc.path.bounds(), own.path.bounds()) else { continue };
            if let (Some(from), Some(to)) = (align_of(p, &pc.name, CMD)?.at(b), align_of(p, &pc.name, CMD)?.at(o)) {
                pc.path.transform(vectorcraft_geom::Affine::translate(to - from));
            }
        }
        // A size the draft and rules can't make keeps the document's outline: say so rather
        // than nest a copy.
        let same = pieces.len() == as_is.len() && pieces.iter().zip(&as_is).all(|(a, b)| a.path == b.path);
        if same && !current {
            warnings.push(format!(
                "size {size} drew the same as the document: does the chart give the measurements the draft uses, or the pieces grade rules?"
            ));
        }
        drawn.push((size.clone(), current, pieces));
    }
    let summary: Vec<Value> = drawn
        .iter()
        .map(|(size, _, pieces)| {
            let pieces: Vec<Value> = pieces
                .iter()
                .map(|pc| {
                    let b = pc.path.bounds().unwrap_or_default();
                    let r = |v: f64| (pt_to_mm(v) * 10.0).round() / 10.0;
                    json!({"name": pc.name, "widthMm": r(b.width()), "heightMm": r(b.height()), "lengthMm": r(pc.path.length())})
                })
                .collect();
            json!({"size": size, "pieces": pieces})
        })
        .collect();
    let (layer, layers) = s.edit("Grade Nest", |d, _| {
        d.layers.retain(|l| l.name.as_deref() != Some(NEST));
        let mut subs = Vec::new();
        let mut ids = Vec::new();
        for (i, (size, current, pieces)) in drawn.iter().enumerate() {
            let (r, g, b) = COLORS[i % COLORS.len()];
            // The size the document is drafted in, heavier.
            let width = if *current { 1.0 } else { 0.5 };
            let look = Appearance::basic(Paint::None, Paint::solid(Color::rgb8(r, g, b)), width);
            let mut children = Vec::new();
            for pc in pieces {
                let mut n = Node::path(d.alloc_id(), pc.path.clone(), look.clone());
                n.name = Some(format!("{} {size}", pc.name));
                children.push(Arc::new(n));
            }
            let mut l = Node::layer(d.alloc_id(), &format!("Size {size}"), LayerColor::Preset((i % 9) as u8));
            ids.push(l.id.0);
            if let Some(c) = l.children_mut() {
                *c = children;
            }
            subs.push(Arc::new(l));
        }
        let mut nest = Node::layer(d.alloc_id(), NEST, LayerColor::Preset(0));
        nest.locked = true;
        let id = nest.id.0;
        if let Some(c) = nest.children_mut() {
            *c = subs;
        }
        d.layers.push(Arc::new(nest));
        Ok((id, ids))
    })?;
    let sizes: Vec<Value> = summary
        .into_iter()
        .zip(layers)
        .map(|(mut v, l)| {
            v["layer"] = json!(l);
            v
        })
        .collect();
    Ok(json!({"layer": layer, "sizes": sizes, "warnings": warnings}))
}

fn clear(s: &mut Session, _: &Value) -> Result<Value> {
    let had = s.doc()?.doc.layers.iter().any(|l| l.name.as_deref() == Some(NEST));
    if had {
        s.edit("Clear Graded Nest", |d, _| {
            d.layers.retain(|l| l.name.as_deref() != Some(NEST));
            Ok(())
        })?;
    }
    Ok(json!({"removed": had}))
}

fn set_sizes(s: &mut Session, p: &Value) -> Result<Value> {
    const CMD: &str = "grade.setSizes";
    let range = match p.get("sizes") {
        Some(Value::Null) => None,
        Some(Value::Array(v)) => {
            let names =
                v.iter().map(|s| s.as_str().map(String::from)).collect::<Option<Vec<_>>>().ok_or_else(|| bad(CMD, "sizes: the sizes' names"))?;
            let base = str_param(p, "base").ok_or_else(|| bad(CMD, "base: the size the pattern is drawn in"))?;
            Some(GradeSizes::new(names, base).map_err(|e| bad(CMD, e.to_string()))?)
        }
        _ => return Err(bad(CMD, "sizes: the sizes' names, smallest first (null removes them)")),
    };
    let out = match &range {
        Some(r) => json!({"sizes": r.sizes, "base": r.base}),
        None => json!({"sizes": null}),
    };
    s.edit("Set Size Range", |d, _| {
        d.grade_sizes = range.map(Box::new);
        Ok(())
    })?;
    Ok(out)
}

/// The one piece a rule command acts on.
fn one_piece(s: &Session, p: &Value, cmd: &str) -> Result<NodeId> {
    match super::piece::target_pieces(s, p, cmd)?.as_slice() {
        [one] => Ok(*one),
        _ => Err(bad(cmd, "select one pattern piece")),
    }
}

fn move_of(v: &Value) -> Option<[f64; 2]> {
    match v.as_array()?.as_slice() {
        [x, y] => Some([x.as_f64()?, y.as_f64()?]),
        _ => None,
    }
}

fn set_rule(s: &mut Session, p: &Value) -> Result<Value> {
    const CMD: &str = "grade.setRule";
    let id = one_piece(s, p, CMD)?;
    let doc = &s.doc()?.doc;
    let n = doc.node(id).ok_or(EngineError::NoNode(id))?;
    let count = seam_of(n).map_or(0, |(_, pts)| pts.len());
    let anchor = match p.get("point") {
        // A point picked on the canvas: the seam line's point nearest it, within PICK_MM.
        Some(v) => {
            let [x, y] = finite_numbers::<2>(v).ok_or_else(|| bad(CMD, "point: [x, y] in document points"))?;
            let pts = seam_of(n).map(|(_, pts)| pts).unwrap_or_default();
            pts.iter()
                .enumerate()
                .map(|(i, q)| (i, (q.x - x).hypot(q.y - y)))
                .filter(|(_, d)| *d <= pf_core::mm_to_pt(PICK_MM))
                .min_by(|a, b| a.1.total_cmp(&b.1))
                .map(|(i, _)| i)
                .ok_or_else(|| bad(CMD, "there is no point of the seam line there"))?
        }
        None => p
            .get("anchor")
            .and_then(Value::as_u64)
            .and_then(|a| usize::try_from(a).ok())
            .ok_or_else(|| bad(CMD, "anchor: the seam line's point, from 0"))?,
    };
    if anchor >= count {
        return Err(bad(CMD, format!("the seam line has points 0 to {}", count.saturating_sub(1))));
    }
    let clear = p.get("clear").and_then(Value::as_bool).unwrap_or(false);
    // across and down, as a dialog gives them: the step.
    let step = match (p.get("across"), p.get("down")) {
        (None, None) => p.get("step").cloned(),
        (dx, dy) => Some(json!([dx.and_then(Value::as_f64).unwrap_or(0.0), dy.and_then(Value::as_f64).unwrap_or(0.0)])),
    };
    let rule = match (step.as_ref(), p.get("sizes")) {
        _ if clear => None,
        (Some(v), None) => Some(Rule::Step(move_of(v).ok_or_else(|| bad(CMD, "step: [dx, dy] in mm"))?)),
        (None, Some(Value::Object(m))) => {
            let mut map = std::collections::BTreeMap::new();
            for (k, v) in m {
                let mv = move_of(v).ok_or_else(|| bad(CMD, format!("{k}: [dx, dy] in mm")))?;
                // Sizes of the range are spelled as the range spells them.
                let name = doc.grade_sizes.as_deref().and_then(|r| r.find(k)).unwrap_or(k.as_str());
                map.insert(name.to_string(), mv);
            }
            Some(Rule::Sizes(map))
        }
        _ => return Err(bad(CMD, "give step: [dx, dy], or sizes: {\"<size>\": [dx, dy]}, or clear: true")),
    };
    let rule = rule.map(|r| r.checked().map_err(|e| bad(CMD, e.to_string()))).transpose()?;
    let mut piece = super::piece::piece_on(n).cloned().ok_or_else(|| bad(CMD, "not a pattern piece"))?;
    piece.grade.retain(|r| r.anchor != anchor);
    if let Some(rule) = rule {
        piece.grade.push(GradeRule { anchor, rule });
        piece.grade.sort_by_key(|r| r.anchor);
    }
    s.edit("Set Grade Rule", |d, _| {
        if let Some(n) = d.node_mut(id) {
            n.extras_mut().pattern = Some(Box::new(pf_core::PatternTag::Piece(Box::new(piece))));
        }
        Ok(())
    })?;
    info_of(&s.doc()?.doc, id)
}

fn info_of(doc: &Document, id: NodeId) -> Result<Value> {
    let n = doc.node(id).ok_or(EngineError::NoNode(id))?;
    let piece = super::piece::piece_on(n).ok_or_else(|| bad("grade.info", "not a pattern piece"))?;
    let rules: Vec<Value> = piece
        .grade
        .iter()
        .map(|r| match &r.rule {
            Rule::Step(m) => json!({"anchor": r.anchor, "step": m}),
            Rule::Sizes(map) => json!({"anchor": r.anchor, "sizes": map}),
        })
        .collect();
    let range = doc.grade_sizes.as_deref();
    let mut moves = serde_json::Map::new();
    if let (Some(r), Some((_, pts))) = (range, seam_of(n)) {
        // The points in mm, as the rules measure.
        let mm: Vec<pf_core::Pt> = pts.iter().map(|q| pf_core::Pt::new(pt_to_mm(q.x), pt_to_mm(q.y))).collect();
        let round = |v: f64| (v * 1000.0).round() / 1000.0;
        for size in &r.sizes {
            let m = pf_core::grade::moves(&mm, &piece.grade, r, size);
            moves.insert(size.clone(), json!(m.iter().map(|v| [round(v[0]), round(v[1])]).collect::<Vec<_>>()));
        }
    }
    Ok(json!({
        "id": id.0,
        "sizes": range.map(|r| r.sizes.clone()),
        "base": range.map(|r| r.base.clone()),
        "rules": rules,
        "moves": moves,
    }))
}

fn info(s: &mut Session, p: &Value) -> Result<Value> {
    let id = one_piece(s, p, "grade.info")?;
    info_of(&s.doc()?.doc, id)
}

fn copy_rules(s: &mut Session, p: &Value) -> Result<Value> {
    const CMD: &str = "grade.copyRules";
    let from = id_param(p, "from").ok_or_else(|| bad(CMD, "from: the pattern piece whose rules are copied"))?;
    let doc = &s.doc()?.doc;
    let from = super::piece::piece_group(doc, from).ok_or_else(|| bad(CMD, "from: not a pattern piece"))?;
    let mirror = match str_param(p, "mirror").unwrap_or("none") {
        "none" => [false, false],
        "x" => [true, false],
        "y" => [false, true],
        "xy" => [true, true],
        _ => return Err(bad(CMD, "mirror: \"none\", \"x\", \"y\" or \"xy\"")),
    };
    let src = doc.node(from).ok_or(EngineError::NoNode(from))?;
    let rules = super::piece::piece_on(src).map(|pc| pc.grade.clone()).unwrap_or_default();
    if rules.is_empty() {
        return Err(bad(CMD, "the piece has no grade rules to copy"));
    }
    let (_, from_pts) = seam_of(src).ok_or_else(|| bad(CMD, "from: the piece has no seam line"))?;
    let q = match p.get("to") {
        Some(v) => json!({ "ids": v }),
        None => json!({}),
    };
    let targets: Vec<NodeId> = super::piece::target_pieces(s, &q, CMD)?.into_iter().filter(|t| *t != from).collect();
    if targets.is_empty() {
        return Err(bad(CMD, "to: the pieces to copy the rules to (select them, or name them)"));
    }
    let doc = &s.doc()?.doc;
    let mut changed = Vec::new();
    for t in &targets {
        let n = doc.node(*t).ok_or(EngineError::NoNode(*t))?;
        let Some((_, pts)) = seam_of(n) else { continue };
        let mut piece = super::piece::piece_on(n).cloned().ok_or_else(|| bad(CMD, "not a pattern piece"))?;
        piece.grade = pf_core::grade::copy_rules(&from_pts, &rules, &pts, mirror);
        changed.push((*t, piece));
    }
    s.edit("Copy Grade Rules", |d, _| {
        for (t, piece) in changed {
            if let Some(n) = d.node_mut(t) {
                n.extras_mut().pattern = Some(Box::new(pf_core::PatternTag::Piece(Box::new(piece))));
            }
        }
        Ok(())
    })?;
    let doc = &s.doc()?.doc;
    let pieces = targets.iter().map(|t| info_of(doc, *t)).collect::<Result<Vec<_>>>()?;
    Ok(json!({ "pieces": pieces }))
}
