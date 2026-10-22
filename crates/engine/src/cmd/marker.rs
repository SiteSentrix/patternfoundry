//! Marker making (PF-9): the pieces cut from one fabric laid out on its width, as closely as
//! they go, to see how much fabric to buy and to cut from.
//!
//! A marker is "a diagram of the precise arrangement of pattern pieces" for one cut, its
//! efficiency the share of the fabric the pieces use
//! (https://www.onlineclothingstudy.com/2011/11/marker-planning-and-marker-making.html). Each copy
//! of a piece (its cut count; a piece cut in mirror-image pairs gives half its copies flipped)
//! is laid with its grainline along the fabric's length, the lengthwise grain, which runs
//! parallel to the selvedge (https://www.threadsmagazine.com/2018/02/22/fabric-grain-explained),
//! and may be laid either way up unless the fabric has a nap or a one-way print, where every
//! piece points the same way ("with nap" layouts, https://www.threadsmagazine.com/2020/06/11/how-to-lay-out-patterns-on-napped-fabrics).
//! The laying itself is [`pf_core::marker::lay`].
//!
//! The marker is drawn on its own layer, "Marker" and the fabric's name: the fabric's outline
//! (selvedges at the top and bottom), and each copy as a group of the piece's cutting line, seam
//! line, grainline and notches with its name, ready to print or export to cut from.

use super::*;
use pf_core::marker::{Part, area, lay};
use pf_core::{Mark, Pt, mm_to_pt, pt_to_mm};
use serde_json::{Value, json};
use std::sync::Arc;
use vectorcraft_color::{Color, Paint};
use vectorcraft_doc::{Appearance, CharStyle, Document, LayerColor, Node, NodeKind, TextObject};
use vectorcraft_geom::{Affine, PathData, Point, SubPath, Vec2};

/// The fabric a piece without one is cut from.
const MAIN: &str = "Main";
/// The default gap between pieces, in mm.
const GAP_MM: f64 = 2.0;
/// The default precision (the grid overlap is tested on), in mm.
const STEP_MM: f64 = 5.0;
/// Most copies one marker lays.
const MAX_COPIES: usize = 2_000;

pub fn specs() -> Vec<CommandSpec> {
    vec![cmd!(
        "marker.make",
        "Make Marker",
        [],
        None,
        "{width: mm (the fabric's width; or a length with its unit, \"150 cm\", \"60 in\"), fabric? (the fabric whose pieces to lay; default: the first piece's; a piece with none is Main), ids? (these pieces instead), gap?: mm (2, between pieces), oneWay?: bool (a nap or one-way print: every piece the same way up), size? (a size of the size range or chart: that size's pieces), precision?: mm (5; 1 to 20, smaller lays closer but takes longer), origin?: [x, y] (document points: the marker's top-left; default below the art)} lays every copy of the pieces (their cut counts, mirror-image pairs flipped) on the fabric with the grainlines along its length, as short as it goes, on the layer \"Marker <fabric>\" (made again it replaces it) → {layer, fabric, widthMm, lengthMm, efficiency (% of the fabric the pieces use), copies, notLaid: [names too wide for the fabric]}",
        has_doc,
        make
    )]
}

/// One copy to lay: the piece's group, which copy it is, and whether it is the mirror image.
struct Copy {
    group: Arc<Node>,
    name: String,
    mirrored: bool,
    /// Grainline along x, mirrored when `mirrored`: from the document into the part's frame.
    base: Affine,
    points: Vec<Pt>,
}

fn fabric_of(piece: &pf_core::Piece) -> &str {
    let f = piece.fabric.trim();
    if f.is_empty() { MAIN } else { f }
}

fn length_mm(p: &Value, k: &str, default: Option<f64>, min: f64, max: f64) -> Result<f64> {
    const C: &str = "marker.make";
    let v = match p.get(k) {
        None | Some(Value::Null) => default.ok_or_else(|| bad(C, format!("{k}: mm, or a length with its unit (\"150 cm\")")))?,
        Some(v) => super::piece::mm_param(v).ok_or_else(|| bad(C, format!("{k}: mm, or a length with its unit (\"150 cm\")")))?,
    };
    if !(min..=max).contains(&v) {
        return Err(bad(C, format!("{k}: {min} to {max} mm")));
    }
    Ok(v)
}

fn make(s: &mut Session, p: &Value) -> Result<Value> {
    const C: &str = "marker.make";
    let width = length_mm(p, "width", None, 10.0, 100_000.0)?;
    let gap = length_mm(p, "gap", Some(GAP_MM), 0.0, 100.0)?;
    let step = length_mm(p, "precision", Some(STEP_MM), 1.0, 20.0)?;
    let one_way = match p.get("oneWay") {
        None | Some(Value::Null) => false,
        Some(v) => v.as_bool().ok_or_else(|| bad(C, "oneWay: true or false"))?,
    };
    let size = match p.get("size") {
        None | Some(Value::Null) => None,
        Some(Value::String(s)) if s.trim().is_empty() => None,
        Some(Value::String(s)) => Some(s.trim().to_string()),
        Some(_) => return Err(bad(C, "size: the name of a size of the size range or chart")),
    };
    // The pieces: those named, else every piece of the fabric.
    let named = if p.get("ids").is_some() || p.get("id").is_some() { Some(super::piece::target_pieces(s, p, C)?) } else { None };
    let own = s.doc()?.doc.clone();
    let doc: Document = match &size {
        Some(size) => super::grade::in_size(&own, size, C)?,
        None => (*own).clone(),
    };
    let mut groups: Vec<Arc<Node>> = Vec::new();
    doc.walk(|n| {
        if super::piece::piece_on(n).is_some() && named.as_ref().is_none_or(|ids| ids.contains(&n.id)) {
            groups.push(Arc::new(n.clone()));
        }
    });
    let fabric = match (str_param(p, "fabric").map(str::trim), &named) {
        (Some(f), _) if !f.is_empty() => f.to_string(),
        _ => groups.first().and_then(|g| super::piece::piece_on(g)).map(|pc| fabric_of(pc).to_string()).unwrap_or_else(|| MAIN.into()),
    };
    if named.is_none() {
        groups.retain(|g| super::piece::piece_on(g).is_some_and(|pc| fabric_of(pc).eq_ignore_ascii_case(&fabric)));
    }
    if groups.is_empty() {
        return Err(bad(C, format!("no pattern pieces of fabric `{fabric}` to lay (Pattern › Make Pattern Piece makes them)")));
    }
    let mut copies: Vec<Copy> = Vec::new();
    for g in &groups {
        let Some(piece) = super::piece::piece_on(g) else { continue };
        let Some(cut) = g.children().into_iter().flatten().find(|c| super::piece::mark_of(c) == Some(Mark::Cut)) else { continue };
        let NodeKind::Path { path, .. } = &cut.kind else { continue };
        let Some((outline, _)) = super::piece::outline_of(path) else { continue };
        let grain = piece.grainline.or_else(|| pf_core::marks::default_grainline(&outline));
        let angle = grain.map_or(0.0, |g| (g.to.y - g.from.y).atan2(g.to.x - g.from.x));
        let seam = g.children().into_iter().flatten().find(|c| super::piece::mark_of(c) == Some(Mark::Seam)).and_then(|c| c.name.as_deref());
        let name = [Some(piece.name.as_str()), seam].into_iter().flatten().map(str::trim).find(|s| !s.is_empty()).unwrap_or("Piece").to_string();
        let n = piece.cut as usize;
        let flipped = if piece.mirror { n / 2 } else { 0 };
        for k in 0..n {
            let mirrored = k >= n - flipped;
            let turn = Affine::rotate(-angle);
            let base = if mirrored { Affine::scale_non_uniform(1.0, -1.0) * turn } else { turn };
            let points = outline
                .points
                .iter()
                .map(|q| {
                    let r = base * Point::new(q.x, q.y);
                    Pt::new(r.x, r.y)
                })
                .collect();
            copies.push(Copy { group: g.clone(), name: name.clone(), mirrored, base, points });
            if copies.len() > MAX_COPIES {
                return Err(bad(C, "too many pieces to lay at once"));
            }
        }
    }
    if copies.is_empty() {
        return Err(bad(C, "the pieces have no cutting lines to lay"));
    }
    let parts: Vec<Part> = copies.iter().map(|c| Part { points: c.points.clone(), turn: !one_way }).collect();
    let layout = lay(&parts, mm_to_pt(width), mm_to_pt(gap), mm_to_pt(step)).map_err(|e| bad(C, e.to_string()))?;
    let used: f64 = parts.iter().zip(&layout.placed).filter(|(_, at)| at.is_some()).map(|(p, _)| area(&p.points)).sum();
    let length = layout.length;
    let efficiency = if length > 0.0 { used / (length * mm_to_pt(width)) * 100.0 } else { 0.0 };
    let mut not_laid: Vec<String> = copies.iter().zip(&layout.placed).filter(|(_, at)| at.is_none()).map(|(c, _)| c.name.clone()).collect();
    not_laid.dedup();
    let origin = match p.get("origin") {
        None | Some(Value::Null) => {
            let b = own.art_bounds();
            let a = own.artboards.first().map_or(Point::ZERO, |a| a.rect.origin());
            Point::new(b.map_or(a.x, |b| b.x0), b.map_or(a.y, |b| b.y1) + mm_to_pt(50.0))
        }
        Some(v) => {
            let [x, y] = finite_numbers::<2>(v).ok_or_else(|| bad(C, "origin: [x, y] in document points"))?;
            Point::new(x, y)
        }
    };
    let layer_name = format!("Marker {fabric}");
    let r1 = |v: f64| (v * 10.0).round() / 10.0;
    let caption = format!(
        "{fabric}{}: {} cm wide, {} cm long, {}% used",
        size.as_deref().map(|s| format!(" (size {s})")).unwrap_or_default(),
        r1(width / 10.0),
        r1(pt_to_mm(length) / 10.0),
        efficiency.round()
    );
    let w_pt = mm_to_pt(width);
    let layer = s.edit("Make Marker", |d, sel| {
        d.layers.retain(|l| l.name.as_deref() != Some(layer_name.as_str()));
        let black = || Paint::solid(Color::BLACK);
        let mut children: Vec<Arc<Node>> = Vec::new();
        // The fabric: its length laid, between the selvedges.
        let corners =
            [origin, Point::new(origin.x + length, origin.y), Point::new(origin.x + length, origin.y + w_pt), Point::new(origin.x, origin.y + w_pt)];
        let mut fab = Node::path(
            d.alloc_id(),
            PathData::single(SubPath::polyline(&corners, true)),
            Appearance::basic(Paint::None, Paint::solid(Color::rgb8(120, 120, 120)), 0.5),
        );
        fab.name = Some(format!("Fabric {fabric}"));
        children.push(Arc::new(fab));
        children.push(Arc::new(text(d, &caption, Point::new(origin.x, origin.y - mm_to_pt(6.0)), false, black())));
        for (c, at) in copies.iter().zip(&layout.placed) {
            let Some(at) = at else { continue };
            let turn = if at.turned { Affine::rotate(std::f64::consts::PI) } else { Affine::IDENTITY };
            let xf = Affine::translate(origin.to_vec2() + Vec2::new(at.offset.x, at.offset.y)) * turn * c.base;
            let mut parts: Vec<Arc<Node>> = Vec::new();
            for child in c.group.children().into_iter().flatten() {
                if !matches!(super::piece::mark_of(child), Some(Mark::Cut | Mark::Seam | Mark::Grainline | Mark::Notches)) {
                    continue;
                }
                let mut n = d.reid(child);
                n.transform(xf, false);
                plain(&mut n);
                parts.push(Arc::new(n));
            }
            // The name, upright, in the middle of the piece.
            let laid: Vec<Pt> = c.points.iter().map(|q| at.apply(*q)).collect();
            let (lo, hi) = laid.iter().fold((Pt::new(f64::INFINITY, f64::INFINITY), Pt::new(f64::NEG_INFINITY, f64::NEG_INFINITY)), |(l, h), q| {
                (Pt::new(l.x.min(q.x), l.y.min(q.y)), Pt::new(h.x.max(q.x), h.y.max(q.y)))
            });
            let mid = Point::new(origin.x + (lo.x + hi.x) / 2.0, origin.y + (lo.y + hi.y) / 2.0);
            parts.push(Arc::new(text(d, &c.name, mid, true, black())));
            let mut g = Node::new(d.alloc_id(), NodeKind::Group { children: parts, clip: false });
            g.name = Some(if c.mirrored { format!("{} (mirrored)", c.name) } else { c.name.clone() });
            children.push(Arc::new(g));
        }
        let mut l = Node::layer(d.alloc_id(), &layer_name, LayerColor::Preset(2));
        let id = l.id;
        if let Some(ch) = l.children_mut() {
            *ch = children;
        }
        d.layers.push(Arc::new(l));
        sel.set([id]);
        Ok(id)
    })?;
    Ok(json!({
        "layer": layer.0,
        "fabric": fabric,
        "widthMm": width,
        "lengthMm": r1(pt_to_mm(length)),
        "efficiency": r1(efficiency),
        "copies": layout.placed.iter().filter(|p| p.is_some()).count(),
        "notLaid": not_laid,
    }))
}

/// A copied part made plain art, unlocked: no longer a piece's generated part, nor a draft or
/// a block (which would be drawn again, unturned, from the measurements).
fn plain(n: &mut Node) {
    n.locked = false;
    if let Some(e) = n.extras_if_any() {
        e.pattern = None;
        e.draft = None;
        e.block = None;
    }
    if let Some(ch) = n.children_mut() {
        for c in ch.iter_mut() {
            plain(Arc::make_mut(c));
        }
    }
}

/// A line of text at `at` (its middle when `centred`, else its left end), 10 pt.
fn text(d: &mut Document, s: &str, at: Point, centred: bool, fill: Paint) -> Node {
    let style = CharStyle { size: 10.0, fill, ..Default::default() };
    let mut t = TextObject::point(Point::ZERO, s, style);
    if centred {
        t.para.justify = vectorcraft_doc::Justify::Center;
    }
    super::typecmd::refresh_bounds(&mut t);
    let b = t.cached_bounds.unwrap_or_else(|| t.estimate_bounds());
    let anchor = if centred { b.center() } else { Point::new(b.x0, b.y1) };
    t.xf = Affine::translate(at.to_vec2() - anchor.to_vec2());
    let mut n = Node::new(d.alloc_id(), NodeKind::Text(Box::new(t)));
    n.appearance = Appearance::basic(Paint::solid(Color::BLACK), Paint::None, 0.0).into();
    n
}
