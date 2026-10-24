//! Draping (PF-10.3): the document's pieces sewn together (`piece.sew`) wrapped round the avatar
//! (`avatar.draw`) at the waist, and let fall onto it as cloth ([`pf_core::cloth`]), for the 3D
//! fit preview. A skirt first: pieces sewn into a ring round the body, drawn upright (their waist
//! at the top, as patterns are).

use super::*;
use pf_core::cloth::{Cloth, Placed, Stiffness, panel};
use pf_core::{Pt, pt_to_mm};
use serde_json::{Value, json};
use vectorcraft_doc::{Document, NodeKind};
use vectorcraft_geom::kurbo::ParamCurveArclen;
use vectorcraft_geom::{Point, SubPath};

/// The default distance between particles, mm.
const STEP_MM: f64 = 15.0;
/// The default number of steps, a sixtieth of a second each: four seconds of falling.
const STEPS: usize = 240;

pub fn specs() -> Vec<CommandSpec> {
    vec![cmd!(
        "drape.make",
        "Drape on Avatar",
        [],
        None,
        "{ids? (the pieces; default every piece sewn to another, piece.sew), precision?: mm (15, the distance between the cloth's particles, 5 to 50), steps? (240: four seconds of falling, 1 to 2000), path? (an .obj file to write the draped cloth to)} wrap the sewn pieces round the avatar (see avatar.draw) at the waist, in the order they are sewn, the first at the front, and let them fall onto it as light woven cloth; pieces are taken as drawn upright, their waist at the top → {pieces, particles, top (mm above the floor: the highest the cloth hangs), hem (mm: its lowest), waist (mm: the avatar's), worstStretch (%), widestSeam (mm), path?}",
        has_doc,
        make
    )]
}

/// The closed seam line of piece `id`.
fn seam_line(doc: &Document, id: NodeId) -> Option<SubPath> {
    let seam = super::piece::seam_of(doc, id)?;
    match &seam.kind {
        NodeKind::Path { path, .. } => path.subpaths.iter().find(|sp| sp.closed).cloned(),
        _ => None,
    }
}

/// `n` points evenly along `sp` (by length) from anchor `from` forward to anchor `to`, in mm.
fn run_points(sp: &SubPath, from: usize, to: usize, n: usize) -> Vec<Pt> {
    let edges = sp.segment_count();
    if edges == 0 || n < 2 {
        return Vec::new();
    }
    let mut segs = Vec::new();
    let mut e = from % edges;
    for _ in 0..edges {
        segs.push((e, sp.segment(e).arclen(1e-6)));
        e = (e + 1) % edges;
        if e == to % edges {
            break;
        }
    }
    let total: f64 = segs.iter().map(|s| s.1).sum();
    (0..n)
        .filter_map(|k| {
            let mut d = total * k as f64 / (n - 1) as f64;
            for &(e, len) in &segs {
                if d <= len + 1e-9 || e == segs[segs.len() - 1].0 {
                    let at = if len > 1e-9 { (d / len).clamp(0.0, 1.0) } else { 0.0 };
                    return super::piece::point_along(sp, e, at).map(|q: Point| Pt::new(pt_to_mm(q.x), pt_to_mm(q.y)));
                }
                d -= len;
            }
            None
        })
        .collect()
}

/// Where polygon `pts` crosses the level `y`: its leftmost and rightmost crossings.
fn span(pts: &[Pt], y: f64) -> Option<(f64, f64)> {
    let n = pts.len();
    let mut out: Option<(f64, f64)> = None;
    for i in 0..n {
        let (a, b) = (pts[i], pts[(i + 1) % n]);
        let (lo, hi) = if a.y <= b.y { (a, b) } else { (b, a) };
        if y < lo.y || y > hi.y {
            continue;
        }
        let x = if (hi.y - lo.y).abs() < 1e-12 { lo.x.min(hi.x) } else { lo.x + (y - lo.y) / (hi.y - lo.y) * (hi.x - lo.x) };
        let xs = if (hi.y - lo.y).abs() < 1e-12 { [lo.x.min(hi.x), lo.x.max(hi.x)] } else { [x, x] };
        for x in xs {
            out = Some(out.map_or((x, x), |(l, r)| (l.min(x), r.max(x))));
        }
    }
    out
}

fn make(s: &mut Session, p: &Value) -> Result<Value> {
    const C: &str = "drape.make";
    let step = match p.get("precision") {
        None | Some(Value::Null) => STEP_MM,
        Some(v) => super::piece::mm_param(v).filter(|m| (5.0..=50.0).contains(m)).ok_or_else(|| bad(C, "precision: 5 to 50 mm"))?,
    };
    let steps = match p.get("steps") {
        None | Some(Value::Null) => STEPS,
        Some(v) => v.as_u64().filter(|n| (1..=2000).contains(n)).ok_or_else(|| bad(C, "steps: 1 to 2000"))? as usize,
    };
    let doc = &s.doc()?.doc;
    let avatar = super::avatar::avatar_of(doc, C)?;
    let runs = super::piece::sewn_runs(doc);
    // The pieces: those named, else every piece with a sewn seam, in the order they are sewn.
    let named: Option<Vec<NodeId>> = if p.get("ids").is_some() { Some(super::piece::target_pieces(s, p, C)?) } else { None };
    let mut pieces: Vec<NodeId> = Vec::new();
    for r in &runs {
        if !pieces.contains(&r.1) && named.as_ref().is_none_or(|ids| ids.contains(&r.1)) {
            pieces.push(r.1);
        }
    }
    if pieces.is_empty() {
        return Err(bad(C, "no pieces are sewn together to drape (piece.sew sews their seams)"));
    }
    // Round the body in the order they are sewn: from the first, on to the piece its next seam
    // is sewn to.
    let mut order = vec![pieces[0]];
    while order.len() < pieces.len() {
        let last = order[order.len() - 1];
        let next = runs
            .iter()
            .filter(|r| r.1 == last)
            .flat_map(|r| runs.iter().filter(move |o| o.0 == r.0 && o.1 != last))
            .map(|o| o.1)
            .find(|id| pieces.contains(id) && !order.contains(id))
            .or_else(|| pieces.iter().copied().find(|id| !order.contains(id)));
        match next {
            Some(id) => order.push(id),
            None => break,
        }
    }
    // Each piece's panel, its seam line, and where it starts on the pattern.
    let mut made = Vec::new();
    let mut outlines = Vec::new();
    for &id in &order {
        let sp = seam_line(doc, id).ok_or_else(|| bad(C, "a sewn piece has no closed seam line"))?;
        let (outline, _) = super::piece::outline_of(&vectorcraft_geom::PathData::single(sp.clone()))
            .ok_or_else(|| bad(C, "a sewn piece has no closed seam line"))?;
        let mm: Vec<Pt> = outline.points.iter().map(|q| Pt::new(pt_to_mm(q.x), pt_to_mm(q.y))).collect();
        let pl = panel(&mm, step).map_err(|e| bad(C, e.to_string()))?;
        let (x0, x1, y0) = mm.iter().fold((f64::INFINITY, f64::NEG_INFINITY, f64::INFINITY), |(a, b, c), q| (a.min(q.x), b.max(q.x), c.min(q.y)));
        made.push((id, sp, pl, x0, x1 - x0, y0));
        outlines.push(mm);
    }
    // Each row of the cloth (by its height below its piece's top) wrapped round a circle as long
    // round as the pieces are wide together at that height, so the sewn sides start together;
    // their tops at the avatar's waist, the first piece's middle at the front.
    let waist = avatar.torso.iter().find(|r| r.name == "waist").map_or(1000.0, |r| r.z);
    let row = |y: f64, y0: f64| ((y - y0) / step).round() as i64;
    // Each row's span across its piece: where the outline crosses the row's height (not the
    // particles' staircase, which jumps a step at a time along a slanted side).
    let extents: Vec<std::collections::BTreeMap<i64, (f64, f64)>> = made
        .iter()
        .zip(&outlines)
        .map(|(m, outline)| {
            let mut e = std::collections::BTreeMap::new();
            for q in &m.2.points {
                e.entry(row(q.y, m.5)).or_insert_with(|| span(outline, q.y).unwrap_or((q.x, q.x)));
            }
            e
        })
        .collect();
    let (extents, count) = (&extents, made.len());
    let width = move |k: usize, r: i64| extents[k].get(&r).map_or(0.0, |(a, b)| b - a);
    let places: Vec<Box<dyn Fn(Pt) -> [f64; 3] + '_>> = (0..count)
        .map(|k| {
            let y0 = made[k].5;
            Box::new(move |q: Pt| {
                let r = row(q.y, y0);
                let total: f64 = (0..count).map(|j| width(j, r)).sum::<f64>().max(step);
                let before: f64 = (0..k).map(|j| width(j, r)).sum::<f64>() - width(0, r) / 2.0;
                let left = extents[k].get(&r).map_or(q.x, |e| e.0);
                let t = 2.0 * std::f64::consts::PI * (before + q.x - left) / total;
                let radius = total / (2.0 * std::f64::consts::PI);
                [radius * t.sin(), radius * t.cos(), waist - (q.y - y0)]
            }) as Box<dyn Fn(Pt) -> [f64; 3] + '_>
        })
        .collect();
    let placed: Vec<Placed> = made.iter().zip(&places).map(|(m, f)| (&m.2, f.as_ref())).collect();
    let mut cloth = Cloth::new(&placed, Stiffness::default());
    // Each seam: its two runs' points paired, the ends level with each other on the pattern
    // matched (the pieces are drawn upright).
    let index = |id: NodeId| order.iter().position(|o| *o == id);
    let labels: std::collections::BTreeSet<&String> = runs.iter().map(|r| &r.0).collect();
    for label in labels {
        let pair: Vec<&(String, NodeId, String, usize, usize)> = runs.iter().filter(|r| &r.0 == label).collect();
        let [a, b] = pair[..] else { continue };
        let (Some(ia), Some(ib)) = (index(a.1), index(b.1)) else { continue };
        let len = |m: &(NodeId, SubPath, pf_core::cloth::Panel, f64, f64, f64), f: usize, t: usize| run_points(&m.1, f, t, 64);
        let (pa, mut pb) = (len(&made[ia], a.3, a.4), len(&made[ib], b.3, b.4));
        let n = pa.len().min(pb.len());
        if n < 2 {
            continue;
        }
        if (pa[0].y - pb[0].y).abs() > (pa[0].y - pb[pb.len() - 1].y).abs() {
            pb.reverse();
        }
        let count = ((pa.iter().zip(pa.iter().skip(1)).map(|(u, v)| (*v - *u).len()).sum::<f64>() / step).ceil() as usize + 1).max(2);
        for k in 0..count {
            let pick = |pts: &[Pt]| pts[(k * (pts.len() - 1)) / (count - 1).max(1)];
            let (qa, qb) = (pick(&pa), pick(&pb));
            if let (Some(x), Some(y)) = (made[ia].2.nearest(qa), made[ib].2.nearest(qb)) {
                cloth.sew(x + cloth.starts[ia], y + cloth.starts[ib]);
            }
        }
    }
    // Four substeps a sixtieth of a second, forty rounds of corrections each: fewer let a ring of
    // cloth stretch round the hips and slide off (at ten, a skirt that fits fell to the floor).
    for _ in 0..steps * 4 {
        cloth.step(1.0 / 240.0, 40, &avatar);
    }
    let (top, hem) = cloth.positions.iter().fold((f64::NEG_INFINITY, f64::INFINITY), |(t, h), q| (t.max(q[2]), h.min(q[2])));
    let r1 = |v: f64| (v * 10.0).round() / 10.0;
    let mut out = json!({
        "pieces": order.len(),
        "particles": cloth.positions.len(),
        "top": r1(top),
        "hem": r1(hem),
        "waist": r1(waist),
        "worstStretch": r1(cloth.worst_stretch() * 100.0),
        "widestSeam": r1(cloth.widest_seam()),
    });
    if let Some(path) = str_param(p, "path") {
        let mesh = pf_core::avatar::Mesh { vertices: cloth.positions.clone(), triangles: cloth.triangles.clone() };
        super::fileio::write_file(path, pf_core::avatar::to_obj(&mesh, "drape").as_bytes())?;
        out["path"] = json!(path);
    }
    Ok(out)
}
