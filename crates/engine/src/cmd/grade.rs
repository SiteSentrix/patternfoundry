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
use pf_core::measure::Measurements;
use pf_core::{Mark, pt_to_mm};
use serde_json::{Value, json};
use std::sync::Arc;
use vectorcraft_color::{Color, Paint};
use vectorcraft_doc::{Appearance, Document, LayerColor, Node, NodeKind};
use vectorcraft_geom::PathData;

/// The nest layer's name; making the nest again replaces the layer of this name.
pub(crate) const NEST: &str = "Graded Nest";

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

/// The pieces' lines of `doc` drafted from `m`.
fn redraft(doc: &Document, m: Measurements, line: Mark, cmd: &str) -> Result<Vec<Drawn>> {
    let mut d = doc.clone();
    d.measurements = Some(Box::new(m));
    // As an edit does: drafted points follow the measurements, then pieces their seam lines.
    super::measure::refresh(&mut d)?;
    super::piece::refresh(doc, &mut d)?;
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
    let chart = doc.size_chart.as_deref().ok_or_else(|| bad(CMD, "the document has no size chart (Measurements › Import Size Chart)"))?;
    let sizes: Vec<String> = match p.get("sizes") {
        None | Some(Value::Null) => chart.size_names().into_iter().map(String::from).collect(),
        Some(Value::Array(v)) => {
            let mut out = Vec::new();
            for s in v {
                let s = s.as_str().ok_or_else(|| bad(CMD, "sizes: the sizes' names"))?;
                let name = chart
                    .size_names()
                    .into_iter()
                    .find(|n| n.eq_ignore_ascii_case(s.trim()))
                    .ok_or_else(|| bad(CMD, format!("the size chart has no size `{s}`")))?;
                out.push(name.to_string());
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
    let as_is = redraft(&base, own.clone(), line, CMD)?;
    let mut drawn = Vec::new();
    let mut warnings = Vec::new();
    for size in &sizes {
        let mut m = own.clone();
        chart.apply(size, &mut m).map_err(|e| bad(CMD, e.to_string()))?;
        // The size the document is drafted in.
        let current = own.size.is_some() && m.size == own.size;
        let mut pieces = redraft(&base, m, line, CMD)?;
        // Lined up on the document's own pieces, at the point asked.
        for (pc, own) in pieces.iter_mut().zip(&as_is) {
            let (Some(b), Some(o)) = (pc.path.bounds(), own.path.bounds()) else { continue };
            if let (Some(from), Some(to)) = (align_of(p, &pc.name, CMD)?.at(b), align_of(p, &pc.name, CMD)?.at(o)) {
                pc.path.transform(vectorcraft_geom::Affine::translate(to - from));
            }
        }
        // A draft the size can't make keeps its last outline: say so rather than nest a copy.
        let same = pieces.len() == as_is.len() && pieces.iter().zip(&as_is).all(|(a, b)| a.path == b.path);
        if same && !current {
            warnings.push(format!("size {size} drew the same as the document: does the chart give the measurements the draft uses?"));
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
