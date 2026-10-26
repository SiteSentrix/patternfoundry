//! The avatar (PF-10.1): a body made from the document's measurements, for the 3D fit preview,
//! drawn front and side at true scale beside the pattern and exported as a mesh for 3D software.
//! The body itself is [`pf_core::avatar`].
//!
//! The measurements are personal data: the avatar is drawn and exported only when asked, and
//! never logged.

use super::*;
use pf_core::avatar::{Avatar, Body, Ring};
use pf_core::mm_to_pt;
use serde_json::{Value, json};
use std::sync::Arc;
use vectorcraft_color::{Color, Paint};
use vectorcraft_doc::{Appearance, Document, LayerColor, Node};
use vectorcraft_geom::{PathData, Point, SubPath};

/// The layer the avatar is drawn on.
const LAYER: &str = "Avatar";

pub fn specs() -> Vec<CommandSpec> {
    vec![
        cmd!(
            "avatar.draw",
            "Draw Avatar",
            ["Pattern"],
            None,
            "{origin?: [x, y] (document points: the top of the front view's neck; default right of the art)} draw the body the document's measurements make (bust, waist, hip, nape_to_waist, waist_to_hip; neck, crotch_depth, waist_to_knee, waist_to_ankle, shoulder, top_arm, sleeve_length, armscye_depth and underbust where the set has them, otherwise from the ANSUR II survey's proportions), seen from the front and from the side at true scale, on the layer \"Avatar\" (drawn again it replaces it) → {layer, front, side, heightMm (to the base of the neck)}",
            has_doc,
            draw
        ),
        cmd!(
            "avatar.exportObj",
            "Export Avatar as OBJ",
            [],
            None,
            "{path? (where to write the .obj file; without it the file comes back as text)} the avatar's mesh (see avatar.draw) as a Wavefront OBJ file in metres, y up, for 3D software → {path | obj, vertices, triangles}",
            has_doc,
            export_obj
        ),
        cmd!(
            "avatar.exportGlb",
            "Export Avatar as glTF",
            [],
            None,
            "{path? (where to write the .glb file; without it the file comes back as dataBase64)} the avatar's mesh (see avatar.draw) as a binary glTF 2.0 file, metres, y up, the front toward +z, for 3D software and game engines (drape.make with a .glb path exports the draped garment on it) → {path | dataBase64, vertices, triangles}",
            has_doc,
            export_glb
        ),
    ]
}

/// The avatar of `doc`'s measurements.
pub(crate) fn avatar_of(doc: &Document, cmd: &str) -> Result<Avatar> {
    let mut env = doc.measurements.as_deref().map(pf_core::measure::Measurements::env).unwrap_or_default();
    let mut need = |name: &'static str| -> Result<f64> {
        env.value(name).ok().filter(|v| *v > 0.0).ok_or_else(|| bad(cmd, pf_core::avatar::AvatarError::Measurement(name).to_string()))
    };
    let (bust, waist, hip, nape_to_waist, waist_to_hip) =
        (need("bust")?, need("waist")?, need("hip")?, need("nape_to_waist")?, need("waist_to_hip")?);
    let mut env = doc.measurements.as_deref().map(pf_core::measure::Measurements::env).unwrap_or_default();
    let mut may = |name: &str| env.value(name).ok().filter(|v| *v > 0.0);
    let body = Body {
        bust,
        waist,
        hip,
        nape_to_waist,
        waist_to_hip,
        neck: may("neck"),
        crotch_depth: may("crotch_depth"),
        waist_to_knee: may("waist_to_knee"),
        waist_to_ankle: may("waist_to_ankle"),
        shoulder: may("shoulder"),
        top_arm: may("top_arm"),
        sleeve_length: may("sleeve_length"),
        armscye_depth: may("armscye_depth"),
        underbust: may("underbust"),
        figure: doc.measurements.as_deref().map(|m| m.figure).unwrap_or_default(),
    };
    pf_core::avatar::avatar(&body).map_err(|e| bad(cmd, e.to_string()))
}

/// The outlines of the body seen from the front (`side` false: across against height) or from the
/// side (along the depth), in mm with z up: the torso and legs, going down one side and up the
/// other, then each arm (seen from the side, one: the other is behind it).
fn silhouette(a: &Avatar, side: bool) -> Vec<Vec<(f64, f64)>> {
    let half = |r: &Ring| {
        let (ax, by) = r.semi_axes();
        if side { (r.y - by, r.y + by) } else { (r.x - ax, r.x + ax) }
    };
    let mut out = Vec::new();
    let torso = &a.torso[..a.torso.len() - 1];
    // Down the right of the torso to the hips…
    out.extend(torso.iter().map(|r| (half(r).1, r.z)));
    if side {
        // …a leg seen from the side, down its front and up its back…
        out.extend(a.legs[0].iter().map(|r| (half(r).1, r.z)));
        out.extend(a.legs[0].iter().rev().map(|r| (half(r).0, r.z)));
    } else {
        // …down the outside of one leg, up its inside, down the inside of the other, up its outside…
        let (right, left) = if a.legs[0][0].x > a.legs[1][0].x { (&a.legs[0], &a.legs[1]) } else { (&a.legs[1], &a.legs[0]) };
        out.extend(right.iter().map(|r| (half(r).1, r.z)));
        out.extend(right.iter().rev().map(|r| (half(r).0, r.z)));
        out.extend(left.iter().map(|r| (half(r).1, r.z)));
        out.extend(left.iter().rev().map(|r| (half(r).0, r.z)));
    }
    // …and up the left of the torso.
    out.extend(torso.iter().rev().map(|r| (half(r).0, r.z)));
    let mut all = vec![out];
    for arm in if side { &a.arms[..1] } else { &a.arms[..] } {
        let mut o: Vec<(f64, f64)> = arm.iter().map(|r| (half(r).1, r.z)).collect();
        o.extend(arm.iter().rev().map(|r| (half(r).0, r.z)));
        all.push(o);
    }
    all
}

fn draw(s: &mut Session, p: &Value) -> Result<Value> {
    const C: &str = "avatar.draw";
    let doc = &s.doc()?.doc;
    let a = avatar_of(doc, C)?;
    let top = a.neck_height;
    let width = |side: bool| silhouette(&a, side).iter().flatten().fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), q| (lo.min(q.0), hi.max(q.0)));
    let (fw, sw) = (width(false), width(true));
    let origin = match p.get("origin") {
        None | Some(Value::Null) => {
            let b = doc.art_bounds();
            let at = doc.artboards.first().map_or(Point::ZERO, |ab| ab.rect.origin());
            Point::new(b.map_or(at.x, |b| b.x1) + mm_to_pt(100.0 - fw.0), b.map_or(at.y, |b| b.y0))
        }
        Some(v) => {
            let [x, y] = finite_numbers::<2>(v).ok_or_else(|| bad(C, "origin: [x, y] in document points"))?;
            Point::new(x, y)
        }
    };
    // Side by side: the side view 100 mm right of the front view.
    let side_x = mm_to_pt(fw.1 + 100.0 - sw.0);
    let line = |side: bool, dx: f64| -> PathData {
        let mut path = PathData::default();
        for outline in silhouette(&a, side) {
            let pts: Vec<Point> = outline.iter().map(|&(u, z)| Point::new(origin.x + dx + mm_to_pt(u), origin.y + mm_to_pt(top - z))).collect();
            path.subpaths.push(SubPath::polyline(&pts, true));
        }
        path
    };
    let (front_path, side_path) = (line(false, 0.0), line(true, side_x));
    let ids = s.edit("Draw Avatar", |d, sel| {
        d.layers.retain(|l| l.name.as_deref() != Some(LAYER));
        let look = || Appearance::basic(Paint::solid(Color::rgb8(232, 222, 210)), Paint::solid(Color::rgb8(120, 110, 100)), 0.75);
        let mut front = Node::path(d.alloc_id(), front_path.clone(), look());
        front.name = Some("Avatar front".into());
        let mut side = Node::path(d.alloc_id(), side_path.clone(), look());
        side.name = Some("Avatar side".into());
        let ids = (front.id, side.id);
        let mut l = Node::layer(d.alloc_id(), LAYER, LayerColor::Preset(3));
        let id = l.id;
        if let Some(ch) = l.children_mut() {
            *ch = vec![Arc::new(front), Arc::new(side)];
        }
        // Below the pattern: the pieces stay on top to be worked on.
        d.layers.insert(0, Arc::new(l));
        sel.set([ids.0, ids.1]);
        Ok((id, ids.0, ids.1))
    })?;
    Ok(json!({"layer": ids.0.0, "front": ids.1.0, "side": ids.2.0, "heightMm": (top * 10.0).round() / 10.0}))
}

fn export_obj(s: &mut Session, p: &Value) -> Result<Value> {
    const C: &str = "avatar.exportObj";
    let a = avatar_of(&s.doc()?.doc, C)?;
    let name = s.doc()?.doc.measurements.as_deref().map(|m| m.name.clone()).filter(|n| !n.trim().is_empty()).unwrap_or_else(|| "avatar".into());
    let obj = pf_core::avatar::to_obj(&a.mesh, &name);
    let counts = (a.mesh.vertices.len(), a.mesh.triangles.len());
    match str_param(p, "path") {
        Some(path) => {
            super::fileio::write_file(path, obj.as_bytes())?;
            Ok(json!({"path": path, "vertices": counts.0, "triangles": counts.1}))
        }
        None => Ok(json!({"obj": obj, "vertices": counts.0, "triangles": counts.1})),
    }
}

fn export_glb(s: &mut Session, p: &Value) -> Result<Value> {
    const C: &str = "avatar.exportGlb";
    use pf_core::gltf::{Part, linear};
    let a = avatar_of(&s.doc()?.doc, C)?;
    // Skin, as the drape view shows it; named only "Avatar" (no one's name in a file meant to be
    // shared).
    let base = [linear(214), linear(192), linear(170), 1.0];
    let part = Part { name: "Avatar", vertices: &a.mesh.vertices, triangles: &a.mesh.triangles, colours: None, base, double_sided: false };
    let bytes = pf_core::gltf::to_glb(&[part]).map_err(|e| bad(C, e.to_string()))?;
    let counts = (a.mesh.vertices.len(), a.mesh.triangles.len());
    match str_param(p, "path") {
        Some(path) => {
            super::fileio::write_file(path, &bytes)?;
            Ok(json!({"path": path, "vertices": counts.0, "triangles": counts.1}))
        }
        None => Ok(json!({"dataBase64": vectorcraft_format::base64_encode(&bytes), "vertices": counts.0, "triangles": counts.1})),
    }
}
