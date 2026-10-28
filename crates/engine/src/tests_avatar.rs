//! The avatar (PF-10.1, `avatar.*`): the body the document's measurements make, drawn front and
//! side at true scale and exported as a mesh.

use pf_core::pt_to_mm;
use serde_json::json;
use vectorcraft_doc::NodeKind;

use super::*;

fn measured() -> Session {
    let mut s = Session::new();
    s.execute("file.new", &json!({"width": 1400, "height": 1000})).unwrap();
    s.execute(
        "measure.set",
        &json!({"values": {"bust": 880, "waist": 680, "hip": 960, "nape_to_waist": 410, "waist_to_hip": 200, "waist_to_ankle": 1040}}),
    )
    .unwrap();
    s
}

/// The avatar `measured` makes (pf-core's own): its crown's height and its feet's.
fn measured_avatar() -> pf_core::avatar::Avatar {
    let body = pf_core::avatar::Body {
        bust: 880.0,
        waist: 680.0,
        hip: 960.0,
        nape_to_waist: 410.0,
        waist_to_hip: 200.0,
        waist_to_ankle: Some(1040.0),
        ..Default::default()
    };
    pf_core::avatar::avatar(&body).unwrap()
}

fn avatar_layers(s: &Session) -> usize {
    s.doc().unwrap().doc.layers.iter().filter(|l| l.name.as_deref() == Some("Avatar")).count()
}

#[test]
fn the_avatar_is_drawn_front_and_side_at_true_scale() {
    let mut s = Session::new();
    s.execute("file.new", &json!({"width": 1400, "height": 1000})).unwrap();
    let e = s.execute("avatar.draw", &json!({})).unwrap_err().to_string();
    assert!(e.contains("bust"), "{e}");
    let mut s = measured();
    let r = s.execute("avatar.draw", &json!({"origin": [0, 0]})).unwrap();
    // The base of the neck is the waist 410 up the back (a little less straight up), the waist
    // 1040 above the ankle bone, the ankle bone the survey's 63 (scaled by the body's height from
    // the waist to the nape over the survey's 416) above the floor: the drawing runs from the top of
    // the head (PF-10.15) to the feet, at a fifth of the ankle bone's height (PF-10.14).
    let height = r["heightMm"].as_f64().unwrap();
    let rise = 410.0 * pf_core::avatar::BACK_RISE;
    let ankle = 63.0 * rise / 416.0;
    assert!((height - (1040.0 + ankle + rise)).abs() < 0.05, "{r}");
    let doc = &s.doc().unwrap().doc;
    let bounds = |id: &serde_json::Value| match &doc.node(NodeId(id.as_u64().unwrap())).unwrap().kind {
        NodeKind::Path { path, .. } => path.bounds().unwrap(),
        _ => panic!("a path"),
    };
    let (front, side) = (bounds(&r["front"]), bounds(&r["side"]));
    for b in [front, side] {
        let a = measured_avatar();
        let feet = a.legs[0].last().unwrap().z;
        assert!((feet - ankle * 0.2).abs() < 1e-6 && a.torso[0].z > height + 150.0);
        assert!((pt_to_mm(b.height()) - (a.torso[0].z - feet)).abs() < 1e-6, "{b:?}");
    }
    let hip = pf_core::avatar::Ring { name: "hip", z: 0.0, x: 0.0, y: 0.0, girth: 960.0, depth_ratio: 0.658 }.semi_axes();
    let bust = pf_core::avatar::Ring { name: "bust", z: 0.0, x: 0.0, y: 0.0, girth: 880.0, depth_ratio: 0.918 }.semi_axes();
    // Front, out to the arms: the wrists hang furthest out (the A pose).
    let body = pf_core::avatar::Body { bust: 880.0, waist: 680.0, hip: 960.0, nape_to_waist: 410.0, waist_to_hip: 200.0, ..Default::default() };
    let a = pf_core::avatar::avatar(&body).unwrap();
    let reach = a.arms.iter().flatten().map(|r| r.x.abs() + r.semi_axes().0).fold(0.0, f64::max);
    assert!(reach > hip.0);
    assert!((pt_to_mm(front.width()) - 2.0 * reach).abs() < 1e-6, "{}", pt_to_mm(front.width()));
    // From the side, from the toes (the feet reach forward, PF-10.14) or the front of the bust (it
    // stands forward) to the furthest back.
    let rings = || a.torso.iter().chain(a.legs.iter().flatten());
    let front_most = rings().map(|r| r.y + r.semi_axes().1).fold(f64::NEG_INFINITY, f64::max);
    let back_most = rings().map(|r| r.y - r.semi_axes().1).fold(f64::INFINITY, f64::min);
    assert!(front_most > bust.1 && (front_most - back_most) > 2.0 * bust.1.max(hip.1));
    assert!((pt_to_mm(side.width()) - (front_most - back_most)).abs() < 1e-6, "{}", pt_to_mm(side.width()));
    assert!(side.x0 > front.x1, "side by side");
    // Drawn again it replaces the last; undone, it goes.
    s.execute("avatar.draw", &json!({})).unwrap();
    assert_eq!(avatar_layers(&s), 1);
    s.execute("edit.undo", &json!({})).unwrap();
    s.execute("edit.undo", &json!({})).unwrap();
    assert_eq!(avatar_layers(&s), 0);
}

#[test]
fn the_avatar_exports_as_an_obj_mesh() {
    let mut s = measured();
    let r = s.execute("avatar.exportObj", &json!({})).unwrap();
    let obj = r["obj"].as_str().unwrap();
    let (v, f) = (r["vertices"].as_u64().unwrap() as usize, r["triangles"].as_u64().unwrap() as usize);
    assert_eq!(obj.lines().filter(|l| l.starts_with("v ")).count(), v);
    assert_eq!(obj.lines().filter(|l| l.starts_with("f ")).count(), f);
    // In metres, y up: the highest point is the top of the head (PF-10.15), 1.7 m or so up.
    let top = obj.lines().filter_map(|l| l.strip_prefix("v ")).filter_map(|l| l.split(' ').nth(1)?.parse::<f64>().ok()).fold(0.0, f64::max);
    assert!((top - measured_avatar().torso[0].z / 1000.0).abs() < 1e-4, "{top}");
    let dir = std::env::temp_dir().join(format!("pf-avatar-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("me.obj");
    let r = s.execute("avatar.exportObj", &json!({"path": path.to_str().unwrap()})).unwrap();
    assert!(r.get("obj").is_none());
    assert_eq!(std::fs::read_to_string(&path).unwrap(), obj);
    std::fs::remove_dir_all(&dir).ok();
}

/// glTF export (PF-10.5): the avatar as a binary glTF file, metres, y up, its triangles facing out.
#[test]
fn the_avatar_exports_as_gltf() {
    let mut s = measured();
    let r = s.execute("avatar.exportGlb", &json!({})).unwrap();
    let bytes = vectorcraft_format::base64_decode(r["dataBase64"].as_str().unwrap()).unwrap();
    let (gltf, bin) = pf_core::gltf::read_glb(&bytes).unwrap();
    let prim = &gltf["meshes"][0]["primitives"][0];
    let acc = |k: &serde_json::Value| &gltf["accessors"][k.as_u64().unwrap() as usize];
    let pos = acc(&prim["attributes"]["POSITION"]);
    assert_eq!(pos["count"].as_u64(), r["vertices"].as_u64());
    assert_eq!(acc(&prim["indices"])["count"].as_u64().unwrap(), r["triangles"].as_u64().unwrap() * 3);
    assert!((pos["max"][1].as_f64().unwrap() - measured_avatar().torso[0].z / 1000.0).abs() < 1e-4, "{pos}");
    // Counter-clockwise seen from outside: the body encloses a positive volume.
    let view = |a: &serde_json::Value| {
        let v = &gltf["bufferViews"][a["bufferView"].as_u64().unwrap() as usize];
        let o = v["byteOffset"].as_u64().unwrap() as usize;
        bin[o..o + v["byteLength"].as_u64().unwrap() as usize].chunks(4).map(|b| [b[0], b[1], b[2], b[3]]).collect::<Vec<_>>()
    };
    let p: Vec<f64> = view(pos).into_iter().map(|b| f32::from_le_bytes(b) as f64).collect();
    let idx: Vec<usize> = view(acc(&prim["indices"])).into_iter().map(|b| u32::from_le_bytes(b) as usize).collect();
    let at = |i: usize| [p[i * 3], p[i * 3 + 1], p[i * 3 + 2]];
    let vol: f64 = idx
        .chunks(3)
        .map(|t| {
            let (a, b, c) = (at(t[0]), at(t[1]), at(t[2]));
            a[0] * (b[1] * c[2] - b[2] * c[1]) - a[1] * (b[0] * c[2] - b[2] * c[0]) + a[2] * (b[0] * c[1] - b[1] * c[0])
        })
        .sum::<f64>()
        / 6.0;
    // Some tens of litres.
    assert!((0.02..0.2).contains(&vol), "{vol}");
    let dir = std::env::temp_dir().join(format!("pf-avatar-glb-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("me.glb");
    s.execute("avatar.exportGlb", &json!({"path": path.to_str().unwrap()})).unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    std::fs::remove_dir_all(&dir).ok();
}

/// The measurement set's figure (PF-10.11) chooses whose proportions the avatar takes: a male
/// one has no bust standing forward, so above the waist it is shallower for the same chest;
/// kept in the document, refused when unknown.
#[test]
fn a_male_figure_is_chosen_on_the_measurements() {
    let mut s = measured();
    assert_eq!(s.execute("measure.info", &json!({})).unwrap()["figure"], json!("female"));
    // How far forward the body comes above the waist (the OBJ mesh: metres, y up, z to the front).
    let width = |s: &mut Session| {
        let r = s.execute("avatar.exportObj", &json!({})).unwrap();
        let obj = r["obj"].as_str().unwrap().to_string();
        obj.lines()
            .filter_map(|l| l.strip_prefix("v "))
            .map(|l| l.split(' ').filter_map(|x| x.parse::<f64>().ok()).collect::<Vec<_>>())
            .filter(|v| v.len() == 3 && v[1] > 1.2)
            .map(|v| v[2] * 1000.0)
            .fold(f64::NEG_INFINITY, f64::max)
    };
    let woman = width(&mut s);
    let r = s.execute("measure.set", &json!({"figure": "male"})).unwrap();
    assert_eq!(r["figure"], json!("male"));
    assert!(width(&mut s) < woman - 5.0);
    // Saved with the document; undone with the edit.
    let saved = serde_json::to_string(s.doc().unwrap().doc.measurements.as_deref().unwrap()).unwrap();
    assert!(saved.contains("\"figure\":\"male\""), "{saved}");
    s.execute("edit.undo", &json!({})).unwrap();
    s.execute("edit.undo", &json!({})).unwrap();
    assert_eq!(s.execute("measure.info", &json!({})).unwrap()["figure"], json!("female"));
    assert!(s.execute("measure.set", &json!({"figure": "tall"})).unwrap_err().to_string().contains("female"));
    // A child (PF-10.13): a six-year-old's measurements make one.
    s.execute(
        "measure.set",
        &json!({"figure": "child", "values": {"bust": 597, "waist": 544, "hip": 616, "nape_to_waist": 282, "waist_to_hip": 108, "waist_to_ankle": null}}),
    )
    .unwrap();
    let r = s.execute("avatar.draw", &json!({"origin": [0, 0]})).unwrap();
    assert!(r["heightMm"].as_f64().unwrap() < 1100.0, "{r}");
}
