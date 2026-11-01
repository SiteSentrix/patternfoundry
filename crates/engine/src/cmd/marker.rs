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
//! A piece cut on the fold ([`pf_core::Piece::fold`]) is laid opened out, whole, on fabric laid
//! out single. On fabric folded lengthwise (`folded`, the usual home-sewing layout, selvedges
//! together: https://www.threadsmagazine.com/2009/06/30/the-fold-line) the marker is half the
//! width with the fold at its top edge: pieces on the fold lie against it, and every other piece
//! cuts a mirror-image pair through both layers, so a piece cut 2 is laid once.
//!
//! The marker is drawn on its own layer, "Marker" and the fabric's name: the fabric's outline
//! (selvedges at the top and bottom), and each copy as a group of the piece's cutting line, seam
//! line, grainline and notches with its name, ready to print or export to cut from.

use super::*;
use pf_core::marker::{Part, area, lay_matched};
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
    vec![
        cmd!(
            "marker.make",
            "Make Marker",
            [],
            None,
            "{width: mm (the fabric's width; or a length with its unit, \"150 cm\", \"60 in\"), fabric? (the fabric whose pieces to lay; default: the first piece's; a piece with none is Main), ids? (these pieces instead), gap?: mm (2, between pieces), oneWay?: bool (a nap or one-way print: every piece the same way up), size? (a size of the size range or chart: that size's pieces), precision?: mm (5; 1 to 20, smaller lays closer but takes longer), folded?: bool (the fabric folded lengthwise, selvedges together: pieces on the fold against the fold, the others laid once a pair; pieces on the fold are opened out otherwise), plaid?: mm | [along, across] (a plaid or check's repeat, mm or lengths with their units: each copy moved only by whole repeats from where its piece is drawn, so pieces drawn level with each other meet the same lines of the plaid, and none turned), origin?: [x, y] (document points: the marker's top-left; default below the art)} lays every copy of the pieces (their cut counts, mirror-image pairs flipped) on the fabric with the grainlines along its length, as short as it goes, on the layer \"Marker <fabric>\" (made again it replaces it) → {layer, fabric, widthMm, lengthMm, efficiency (% of the fabric the pieces use), copies, notLaid: [names too wide for the fabric], warnings}",
            has_doc,
            make
        ),
        cmd!(
            "marker.check",
            "Check Marker",
            [],
            None,
            "{layer? (a marker's layer id; default: the marker the selection is on, or the document's only one)} after the marker's pieces are moved by hand: the fabric's length drawn again to the last piece and the caption's length and % used brought up to date; copies that overlap and copies off the fabric (past a selvedge, or before its start) are listed → {layer, lengthMm, efficiency, copies, overlaps: [[name, name]], offFabric: [name]}",
            has_doc,
            check
        ),
    ]
}

/// One copy to lay: the piece's group, which copy it is, and whether it is the mirror image.
struct Copy {
    group: Arc<Node>,
    name: String,
    mirrored: bool,
    /// Grainline along x, mirrored when `mirrored`: from the document into the part's frame.
    base: Affine,
    points: Vec<Pt>,
    /// Opened out across the fold: the whole cutting line (document points) and the reflection
    /// that draws the other half.
    opened: Option<(Vec<Pt>, Affine)>,
    /// Laid on folded fabric, it cuts a mirror-image pair.
    pair: bool,
    /// Laid against the fold of folded fabric.
    on_fold: bool,
}

/// The reflection across the line through `a` and `b`.
fn reflection(a: Point, b: Point) -> Affine {
    let t = (b.y - a.y).atan2(b.x - a.x);
    Affine::translate(a.to_vec2()) * Affine::rotate(t) * Affine::scale_non_uniform(1.0, -1.0) * Affine::rotate(-t) * Affine::translate(-a.to_vec2())
}

/// Points `pts` under `xf`.
fn mapped(xf: Affine, pts: &[Pt]) -> Vec<Pt> {
    pts.iter()
        .map(|q| {
            let r = xf * Point::new(q.x, q.y);
            Pt::new(r.x, r.y)
        })
        .collect()
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
    let folded = match p.get("folded") {
        None | Some(Value::Null) => false,
        Some(v) => v.as_bool().ok_or_else(|| bad(C, "folded: true or false"))?,
    };
    // A plaid's repeat along the fabric and across it.
    let plaid = match p.get("plaid") {
        None | Some(Value::Null) => None,
        Some(Value::String(t)) if t.trim().is_empty() => None,
        Some(Value::Array(a)) if a.len() == 2 => {
            let mm = |v: &Value| super::piece::mm_param(v).filter(|m| *m >= 1.0 && *m <= 10_000.0);
            match (mm(&a[0]), mm(&a[1])) {
                (Some(x), Some(y)) => Some((x, y)),
                _ => return Err(bad(C, "plaid: the repeat along and across, 1 to 10 000 mm each")),
            }
        }
        Some(v) => match super::piece::mm_param(v).filter(|m| *m >= 1.0 && *m <= 10_000.0) {
            Some(m) => Some((m, m)),
            None => return Err(bad(C, "plaid: the repeat (mm, or [along, across])")),
        },
    };
    // Folded, the marker is half the fabric's width.
    let laid_width = if folded { width / 2.0 } else { width };
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
    let mut warnings: Vec<String> = Vec::new();
    for g in &groups {
        let Some(piece) = super::piece::piece_on(g) else { continue };
        let Some(cut) = g.children().into_iter().flatten().find(|c| super::piece::mark_of(c) == Some(Mark::Cut)) else { continue };
        let NodeKind::Path { path, .. } = &cut.kind else { continue };
        let Some((outline, _)) = super::piece::outline_of(path) else { continue };
        let grain = piece.grainline.or_else(|| pf_core::marks::default_grainline(&outline));
        let angle = grain.map_or(0.0, |g| (g.to.y - g.from.y).atan2(g.to.x - g.from.x));
        let seam = g.children().into_iter().flatten().find(|c| super::piece::mark_of(c) == Some(Mark::Seam));
        let name = [Some(piece.name.as_str()), seam.and_then(|c| c.name.as_deref())]
            .into_iter()
            .flatten()
            .map(str::trim)
            .find(|s| !s.is_empty())
            .unwrap_or("Piece")
            .to_string();
        // The fold line: the seam line's fold edge, which the cutting line runs along.
        let fold = piece.fold.and_then(|e| match seam.map(|n| &n.kind) {
            Some(NodeKind::Path { path, .. }) => path.subpaths.iter().find(|sp| sp.closed).filter(|sp| e < sp.segment_count()).map(|sp| {
                let seg = sp.segment(e);
                (seg.p0, seg.p3)
            }),
            _ => None,
        });
        let n = piece.cut as usize;
        let turn = Affine::rotate(-angle);
        let add = |c: Copy, copies: &mut Vec<Copy>| -> Result<()> {
            copies.push(c);
            if copies.len() > MAX_COPIES {
                return Err(bad(C, "too many pieces to lay at once"));
            }
            Ok(())
        };
        let copy = |mirrored: bool, base: Affine, points: Vec<Pt>, opened: Option<(Vec<Pt>, Affine)>, pair: bool| Copy {
            group: g.clone(),
            name: name.clone(),
            mirrored,
            base,
            points,
            opened,
            pair,
            on_fold: false,
        };
        match fold {
            Some((a, b)) if folded => {
                // Against the fold: the fold edge along x, pointing as the grainline does, the
                // piece below it.
                let mut d = b - a;
                if let Some(g) = grain
                    && d.x * (g.to.x - g.from.x) + d.y * (g.to.y - g.from.y) < 0.0
                {
                    d = -d;
                }
                let mut base = Affine::rotate(-d.y.atan2(d.x));
                let line = (base * a).y;
                let pts = mapped(base, &outline.points);
                if pts.iter().map(|q| q.y - line).sum::<f64>() < 0.0 {
                    base = Affine::scale_non_uniform(1.0, -1.0) * base;
                }
                for _ in 0..n {
                    add(Copy { on_fold: true, ..copy(false, base, mapped(base, &outline.points), None, false) }, &mut copies)?;
                }
            }
            Some((a, b)) => match pf_core::marker::opened_out(&outline.points, Pt::new(a.x, a.y), Pt::new(b.x, b.y)) {
                Some(whole) => {
                    for _ in 0..n {
                        add(copy(false, turn, mapped(turn, &whole), Some((whole.clone(), reflection(a, b))), false), &mut copies)?;
                    }
                }
                None => {
                    warnings.push(format!("{name}: its cutting line doesn't run along the fold edge, so it is laid as it is, not opened out"));
                    for _ in 0..n {
                        add(copy(false, turn, mapped(turn, &outline.points), None, false), &mut copies)?;
                    }
                }
            },
            None if folded => {
                // Through both layers each copy cuts a pair.
                if n % 2 == 1 {
                    warnings.push(format!("{name}: cut {n} on folded fabric cuts {} (pairs)", n + 1));
                }
                for _ in 0..n.div_ceil(2) {
                    add(copy(false, turn, mapped(turn, &outline.points), None, true), &mut copies)?;
                }
            }
            None => {
                let flipped = if piece.mirror { n / 2 } else { 0 };
                for k in 0..n {
                    let mirrored = k >= n - flipped;
                    let base = if mirrored { Affine::scale_non_uniform(1.0, -1.0) * turn } else { turn };
                    add(copy(mirrored, base, mapped(base, &outline.points), None, false), &mut copies)?;
                }
            }
        }
    }
    if copies.is_empty() {
        return Err(bad(C, "the pieces have no cutting lines to lay"));
    }
    let parts: Vec<Part> = copies.iter().map(|c| Part { points: c.points.clone(), turn: !one_way && plaid.is_none(), fold: c.on_fold }).collect();
    let repeat = plaid.map(|(x, y)| Pt::new(mm_to_pt(x), mm_to_pt(y)));
    let layout = lay_matched(&parts, mm_to_pt(laid_width), mm_to_pt(gap), mm_to_pt(step), repeat).map_err(|e| bad(C, e.to_string()))?;
    let used: f64 = parts.iter().zip(&layout.placed).filter(|(_, at)| at.is_some()).map(|(p, _)| area(&p.points)).sum();
    let length = layout.length;
    let efficiency = if length > 0.0 { used / (length * mm_to_pt(laid_width)) * 100.0 } else { 0.0 };
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
        "{fabric}{}: {} cm wide{}{}, {} cm long, {}% used",
        size.as_deref().map(|s| format!(" (size {s})")).unwrap_or_default(),
        r1(width / 10.0),
        if folded { " folded" } else { "" },
        plaid.map(|(x, y)| format!(", plaid {} × {} cm", r1(x / 10.0), r1(y / 10.0))).unwrap_or_default(),
        r1(pt_to_mm(length) / 10.0),
        efficiency.round()
    );
    let w_pt = mm_to_pt(laid_width);
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
        if folded {
            // The fold, along the top edge, dashed.
            let mut a = Appearance::basic(Paint::None, black(), 1.0);
            if let Some(st) = a.stroke_mut() {
                st.dash = Some(vectorcraft_doc::Dash { pattern: vec![8.0, 4.0], ..Default::default() });
            }
            let mut f = Node::path(d.alloc_id(), PathData::single(SubPath::polyline(&[origin, Point::new(origin.x + length, origin.y)], false)), a);
            f.name = Some("Fold".into());
            children.push(Arc::new(f));
        }
        for (c, at) in copies.iter().zip(&layout.placed) {
            let Some(at) = at else { continue };
            let turn = if at.turned { Affine::rotate(std::f64::consts::PI) } else { Affine::IDENTITY };
            let xf = Affine::translate(origin.to_vec2() + Vec2::new(at.offset.x, at.offset.y)) * turn * c.base;
            let mut parts: Vec<Arc<Node>> = Vec::new();
            for child in c.group.children().into_iter().flatten() {
                if !matches!(super::piece::mark_of(child), Some(Mark::Cut | Mark::Seam | Mark::Grainline | Mark::Notches | Mark::Drills)) {
                    continue;
                }
                let cutting = super::piece::mark_of(child) == Some(Mark::Cut);
                match &c.opened {
                    // Opened out: the whole cutting line once, the rest on both halves.
                    Some((whole, _)) if cutting => {
                        let mut n = d.reid(child);
                        let ring: Vec<Point> = whole.iter().map(|q| Point::new(q.x, q.y)).collect();
                        if let NodeKind::Path { path, .. } = &mut n.kind {
                            *path = PathData::single(SubPath::polyline(&ring, true));
                        }
                        n.transform(xf, false);
                        plain(&mut n);
                        parts.push(Arc::new(n));
                    }
                    Some((_, mirror)) => {
                        for m in [Affine::IDENTITY, *mirror] {
                            let mut n = d.reid(child);
                            n.transform(xf * m, false);
                            plain(&mut n);
                            parts.push(Arc::new(n));
                        }
                    }
                    None => {
                        let mut n = d.reid(child);
                        n.transform(xf, false);
                        plain(&mut n);
                        parts.push(Arc::new(n));
                    }
                }
            }
            // The name, upright, in the middle of the piece.
            let laid: Vec<Pt> = c.points.iter().map(|q| at.apply(*q)).collect();
            let (lo, hi) = laid.iter().fold((Pt::new(f64::INFINITY, f64::INFINITY), Pt::new(f64::NEG_INFINITY, f64::NEG_INFINITY)), |(l, h), q| {
                (Pt::new(l.x.min(q.x), l.y.min(q.y)), Pt::new(h.x.max(q.x), h.y.max(q.y)))
            });
            let mid = Point::new(origin.x + (lo.x + hi.x) / 2.0, origin.y + (lo.y + hi.y) / 2.0);
            parts.push(Arc::new(text(d, &c.name, mid, true, black())));
            let mut g = Node::new(d.alloc_id(), NodeKind::Group { children: parts, clip: false });
            g.name = Some(match (c.mirrored, c.pair) {
                (true, _) => format!("{} (mirrored)", c.name),
                (_, true) => format!("{} (pair)", c.name),
                _ => c.name.clone(),
            });
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
        "folded": folded,
        "plaid": plaid.map(|(x, y)| json!([r1(x), r1(y)])),
        "warnings": warnings,
    }))
}

/// The marker layer `p` names, or the one the selection is on, or the document's only one.
fn marker_layer(s: &Session, p: &Value, cmd: &str) -> Result<NodeId> {
    let st = s.doc()?;
    let is_marker = |id: NodeId| st.doc.layers.iter().any(|l| l.id == id && l.name.as_deref().is_some_and(|n| n.starts_with("Marker ")));
    if let Some(v) = p.get("layer").filter(|v| !v.is_null()) {
        let id = NodeId(v.as_u64().ok_or_else(|| bad(cmd, "layer: a marker layer's id"))?);
        return if is_marker(id) { Ok(id) } else { Err(bad(cmd, "not a marker's layer (Pattern › Make Marker…)")) };
    }
    let mut on = st.selection.in_paint_order(&st.doc).into_iter().filter_map(|id| {
        let mut at = id;
        loop {
            if is_marker(at) {
                return Some(at);
            }
            at = st.doc.parent_of(at)?;
        }
    });
    if let Some(id) = on.next() {
        return Ok(id);
    }
    let all: Vec<NodeId> = st.doc.layers.iter().map(|l| l.id).filter(|id| is_marker(*id)).collect();
    match all[..] {
        [one] => Ok(one),
        [] => Err(bad(cmd, "the document has no marker (Pattern › Make Marker…)")),
        _ => Err(bad(cmd, "select a piece on the marker to check")),
    }
}

/// The corners of a path's first outline, in document points (its anchors: cutting lines are
/// polylines).
fn outline_of(n: &Node) -> Vec<Pt> {
    match &n.kind {
        NodeKind::Path { path, .. } => path.subpaths.first().map(|sp| sp.anchors.iter().map(|a| Pt::new(a.p.x, a.p.y)).collect()).unwrap_or_default(),
        _ => Vec::new(),
    }
}

fn check(s: &mut Session, p: &Value) -> Result<Value> {
    const C: &str = "marker.check";
    let layer = marker_layer(s, p, C)?;
    let doc = &s.doc()?.doc;
    let l = doc.node(layer).ok_or(EngineError::NoNode(layer))?;
    let children = l.children().map(|c| c.to_vec()).unwrap_or_default();
    let fabric = children
        .iter()
        .find(|c| c.name.as_deref().is_some_and(|n| n.starts_with("Fabric ")))
        .ok_or_else(|| bad(C, "the marker has lost its fabric outline"))?;
    let corners = outline_of(fabric);
    let (x0, y0, y1) = corners.iter().fold((f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY), |(a, b, c), q| (a.min(q.x), b.min(q.y), c.max(q.y)));
    if !(x0.is_finite() && y1 > y0) {
        return Err(bad(C, "the marker has lost its fabric outline"));
    }
    // Each copy: its name and its cutting line.
    let copies: Vec<(String, Vec<Pt>)> = children
        .iter()
        .filter(|c| matches!(c.kind, NodeKind::Group { .. }))
        .filter_map(|g| {
            let cut = g.children()?.iter().find(|c| c.name.as_deref() == Some("Cutting line"))?;
            let pts = outline_of(cut);
            (pts.len() >= 3).then(|| (g.name.clone().unwrap_or_default(), pts))
        })
        .collect();
    if copies.is_empty() {
        return Err(bad(C, "the marker has no pieces"));
    }
    let e = mm_to_pt(0.05);
    let end = copies.iter().flat_map(|(_, pts)| pts.iter().map(|q| q.x)).fold(x0, f64::max);
    let length = end - x0;
    let width = y1 - y0;
    let used: f64 = copies.iter().map(|(_, pts)| area(pts)).sum();
    let efficiency = if length > 0.0 { used / (length * width) * 100.0 } else { 0.0 };
    let off: Vec<String> =
        copies.iter().filter(|(_, pts)| pts.iter().any(|q| q.y < y0 - e || q.y > y1 + e || q.x < x0 - e)).map(|(n, _)| n.clone()).collect();
    let mut overlaps = Vec::new();
    for i in 0..copies.len() {
        for j in i + 1..copies.len() {
            if pf_core::marker::overlaps(&copies[i].1, &copies[j].1) {
                overlaps.push(json!([copies[i].0, copies[j].0]));
            }
        }
    }
    let r1 = |v: f64| (v * 10.0).round() / 10.0;
    let fold = children.iter().find(|c| c.name.as_deref() == Some("Fold")).map(|c| c.id);
    let (fabric_id, caption) = (fabric.id, children.iter().find(|c| matches!(c.kind, NodeKind::Text(_))).map(|c| c.id));
    let new_caption = caption.and_then(|id| match &doc.node(id)?.kind {
        NodeKind::Text(t) => {
            let old = t.plain_text();
            // "<fabric>…: W cm wide…, L cm long, E% used": the length and the share again.
            let head = old.rsplit_once(", ").and_then(|(h, _)| h.rsplit_once(", ")).map(|(h, _)| h.to_string())?;
            Some(format!("{head}, {} cm long, {}% used", r1(pt_to_mm(length) / 10.0), efficiency.round()))
        }
        _ => None,
    });
    let at = Point::new(x0, y0 - mm_to_pt(6.0));
    s.edit("Check Marker", |d, _| {
        let ring = [Point::new(x0, y0), Point::new(x0 + length, y0), Point::new(x0 + length, y1), Point::new(x0, y1)];
        if let Some(n) = d.node_mut(fabric_id)
            && let NodeKind::Path { path, .. } = &mut n.kind
        {
            *path = PathData::single(SubPath::polyline(&ring, true));
        }
        // Folded fabric: the fold along the top edge, as long.
        if let Some(id) = fold
            && let Some(n) = d.node_mut(id)
            && let NodeKind::Path { path, .. } = &mut n.kind
        {
            *path = PathData::single(SubPath::polyline(&ring[..2], false));
        }
        if let (Some(id), Some(text_now)) = (caption, &new_caption) {
            let mut fresh = text(d, text_now, at, false, Paint::solid(Color::BLACK));
            fresh.id = id;
            if let Some(n) = d.node_mut(id) {
                *n = fresh;
            }
        }
        Ok(())
    })?;
    Ok(json!({
        "layer": layer.0,
        "lengthMm": r1(pt_to_mm(length)),
        "efficiency": r1(efficiency),
        "copies": copies.len(),
        "overlaps": overlaps,
        "offFabric": off,
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
