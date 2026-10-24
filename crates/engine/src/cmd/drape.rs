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

/// The default distance between particles, mm: finer needs many more corrections a step to hold
/// the cloth's length (a skirt at 10 mm and the same 40 slid off), and takes longer.
const STEP_MM: f64 = 20.0;
/// The default number of steps, a sixtieth of a second each: four seconds of falling.
const STEPS: usize = 240;

pub fn specs() -> Vec<CommandSpec> {
    vec![cmd!(
        "drape.make",
        "Drape on Avatar",
        [],
        None,
        "{ids? (the pieces; default every piece sewn to another, piece.sew), precision?: mm (20, the distance between the cloth's particles, 15 to 50), steps? (240: four seconds of falling, 1 to 2000), path? (a file to write the draped cloth to: .glb, a binary glTF 2.0 file of the avatar and the cloth in the drape view's colours, metres, y up, the front toward +z, for 3D software and game engines; else an OBJ file of the cloth), format?: obj|glb (default from path's extension; without a path the file comes back as data (obj) or dataBase64 (glb)), draw?: true (the drape view: the avatar and the draped cloth as pictures seen from the front and the left side, at true scale, the cloth coloured blue where it is loose, green, and red where it is 10% or more larger than cut, on the layer \"Drape\", drawn again it replaces it; false: none), origin?: [x, y] (document points: the front view's top-left; default right of the art)} wrap the sewn pieces round the avatar (see avatar.draw) at the waist, in the order they are sewn, the first at the front, and let them fall onto it as light woven cloth; pieces are taken as drawn upright, their waist at the top → {pieces, particles, top (mm above the floor: the highest the cloth hangs), hem (mm: its lowest), waist (mm: the avatar's), worstStretch (%), widestSeam (mm), path?, data?, dataBase64?, layer?, front?, side?, tightest (%: the most a triangle of cloth is larger than cut)}",
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
        Some(v) => super::piece::mm_param(v).filter(|m| (15.0..=50.0).contains(m)).ok_or_else(|| bad(C, "precision: 15 to 50 mm"))?,
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
    // Four substeps a sixtieth of a second, forty rounds of corrections each at the default
    // precision, more the finer it is (the cloth gets softer the more particles there are): fewer
    // let a ring of cloth stretch round the hips and slide off (at ten, a skirt that fits fell to
    // the floor).
    for _ in 0..steps * 4 {
        cloth.step(1.0 / 240.0, (40.0 * (20.0 / step).powi(2)).round() as usize, &avatar);
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
    let tight = cloth.tightness();
    out["tightest"] = json!(r1(tight.iter().copied().fold(0.0, f64::max) * 100.0));
    // The draped cloth as a file: OBJ (the cloth), or glTF (the avatar and the cloth in its
    // colours), by `format`, else the path's extension.
    let path = str_param(p, "path");
    let format = match (str_param(p, "format"), path) {
        (Some(f), _) => Some(f.to_ascii_lowercase()),
        (None, Some(path)) => Some(if super::fileio::extension(path).eq_ignore_ascii_case("glb") { "glb".into() } else { "obj".into() }),
        (None, None) => None,
    };
    if let Some(format) = format {
        let bytes = match format.as_str() {
            "obj" => {
                let mesh = pf_core::avatar::Mesh { vertices: cloth.positions.clone(), triangles: cloth.triangles.clone() };
                pf_core::avatar::to_obj(&mesh, "drape").into_bytes()
            }
            "glb" => glb(&avatar, &cloth, &tight).map_err(|e| bad(C, e.to_string()))?,
            _ => return Err(bad(C, "format: \"obj\" or \"glb\"")),
        };
        match path {
            Some(path) => {
                super::fileio::write_file(path, &bytes)?;
                out["path"] = json!(path);
            }
            None if format == "obj" => out["data"] = json!(String::from_utf8_lossy(&bytes)),
            None => out["dataBase64"] = json!(vectorcraft_format::base64_encode(&bytes)),
        }
    }
    if p.get("draw").and_then(Value::as_bool) != Some(false) {
        let (layer, front, side) = draw_views(s, p, &avatar, &cloth, &tight)?;
        out["layer"] = json!(layer.0);
        out["front"] = json!(front.0);
        out["side"] = json!(side.0);
    }
    Ok(out)
}

/// The skin's colour, in the drape view and the glTF file.
const SKIN: [u8; 3] = [214, 192, 170];

/// Each particle's colour: the tightness of the triangles round it, averaged.
fn particle_colours(cloth: &Cloth, tight: &[f64]) -> Vec<[u8; 3]> {
    let mut sum = vec![(0.0, 0usize); cloth.positions.len()];
    for (t, tri) in tight.iter().zip(&cloth.triangles) {
        for i in tri {
            if let Some(s) = sum.get_mut(*i as usize) {
                s.0 += t;
                s.1 += 1;
            }
        }
    }
    sum.iter().map(|(t, n)| pf_core::view3d::tightness_colour(if *n > 0 { t / *n as f64 } else { 0.0 })).collect()
}

/// The avatar and the draped cloth as a binary glTF file, the cloth coloured by how tight it is
/// (as the drape view colours it).
fn glb(avatar: &pf_core::avatar::Avatar, cloth: &Cloth, tight: &[f64]) -> std::result::Result<Vec<u8>, pf_core::gltf::GltfError> {
    use pf_core::gltf::{Part, linear, linear_rgb, to_glb};
    let colours: Vec<[u8; 3]> = particle_colours(cloth, tight).into_iter().map(linear_rgb).collect();
    let skin = [linear(SKIN[0]), linear(SKIN[1]), linear(SKIN[2]), 1.0];
    to_glb(&[
        Part { name: "Avatar", vertices: &avatar.mesh.vertices, triangles: &avatar.mesh.triangles, colours: None, base: skin, double_sided: false },
        Part { name: "Drape", vertices: &cloth.positions, triangles: &cloth.triangles, colours: Some(&colours), base: [1.0; 4], double_sided: true },
    ])
}

/// The layer the drape view is drawn on.
const LAYER: &str = "Drape";
/// The drape view's pixel, mm: fine enough to see folds at true scale on the page.
const PIXEL_MM: f64 = 2.0;

/// A picture as PNG.
fn png(pic: &pf_core::view3d::Picture) -> Result<Vec<u8>> {
    let img = image::RgbaImage::from_raw(pic.width, pic.height, pic.rgba.clone())
        .ok_or_else(|| EngineError::Other("the drape view's picture is the wrong size".into()))?;
    let mut buf = Vec::new();
    img.write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png).map_err(|e| EngineError::Other(format!("the drape view: {e}")))?;
    Ok(buf)
}

/// Draw the avatar and the draped cloth seen from the front and the left side, side by side at
/// true scale, on the layer "Drape" (replacing it): → (layer, front, side).
fn draw_views(s: &mut Session, p: &Value, avatar: &pf_core::avatar::Avatar, cloth: &Cloth, tight: &[f64]) -> Result<(NodeId, NodeId, NodeId)> {
    use pf_core::view3d::{Shape, Side, draw, tightness_colour};
    const C: &str = "drape.make";
    let skin = vec![SKIN; avatar.mesh.triangles.len()];
    let colours: Vec<[u8; 3]> = tight.iter().map(|t| tightness_colour(*t)).collect();
    let shapes = [
        Shape { points: &avatar.mesh.vertices, triangles: &avatar.mesh.triangles, colours: &skin },
        Shape { points: &cloth.positions, triangles: &cloth.triangles, colours: &colours },
    ];
    let (front, side) = (draw(&shapes, Side::Front, PIXEL_MM), draw(&shapes, Side::Left, PIXEL_MM));
    let (Some(front), Some(side)) = (front, side) else { return Err(bad(C, "nothing to draw")) };
    let doc = &s.doc()?.doc;
    let origin = match p.get("origin") {
        None | Some(Value::Null) => {
            let b = doc.art_bounds();
            let at = doc.artboards.first().map_or(Point::ZERO, |ab| ab.rect.origin());
            Point::new(b.map_or(at.x, |b| b.x1) + pf_core::mm_to_pt(100.0), b.map_or(at.y, |b| b.y0))
        }
        Some(v) => {
            let [x, y] = finite_numbers::<2>(v).ok_or_else(|| bad(C, "origin: [x, y] in document points"))?;
            Point::new(x, y)
        }
    };
    let (fpng, spng) = (png(&front)?, png(&side)?);
    // The side view 100 mm right of the front view; both from the same height (their tops are
    // the same point of the body), so they line up across.
    let side_x = pf_core::mm_to_pt(front.width as f64 * front.mm + 100.0);
    let dy = pf_core::mm_to_pt(front.top - side.top);
    s.edit("Drape on Avatar", |d, sel| {
        d.layers.retain(|l| l.name.as_deref() != Some(LAYER));
        let image = |d: &mut Document, pic: &pf_core::view3d::Picture, bytes: Vec<u8>, at: Point, name: &str| {
            let key = super::menucmds::unique_key(d, "drape");
            d.images.insert(key.clone(), vectorcraft_doc::ImageBlob::new("image/png", bytes));
            let k = pf_core::mm_to_pt(pic.mm);
            let xf = vectorcraft_geom::Affine::new([k, 0.0, 0.0, k, at.x, at.y]);
            let mut n = vectorcraft_doc::Node::new(
                d.alloc_id(),
                NodeKind::Image(Box::new(vectorcraft_doc::ImageObject {
                    key,
                    width: pic.width,
                    height: pic.height,
                    xf,
                    link: None,
                    placement: Default::default(),
                })),
            );
            n.name = Some(name.into());
            n
        };
        let f = image(d, &front, fpng.clone(), origin, "Drape front");
        let sd = image(d, &side, spng.clone(), Point::new(origin.x + side_x, origin.y + dy), "Drape side");
        let ids = (f.id, sd.id);
        let mut l = vectorcraft_doc::Node::layer(d.alloc_id(), LAYER, vectorcraft_doc::LayerColor::Preset(4));
        let id = l.id;
        if let Some(ch) = l.children_mut() {
            *ch = vec![std::sync::Arc::new(f), std::sync::Arc::new(sd)];
        }
        d.layers.push(std::sync::Arc::new(l));
        sel.set([ids.0, ids.1]);
        Ok((id, ids.0, ids.1))
    })
}
