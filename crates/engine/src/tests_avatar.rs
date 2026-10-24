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
    // The base of the neck is the waist 410 up, the waist 1040 above the ankle bone, the ankle
    // bone the survey's 63 (scaled by 410 / 416) above the floor: the drawing runs from the neck
    // to the ankle.
    let height = r["heightMm"].as_f64().unwrap();
    let ankle = 63.0 * 410.0 / 416.0;
    assert!((height - (1040.0 + ankle + 410.0)).abs() < 0.05, "{r}");
    let doc = &s.doc().unwrap().doc;
    let bounds = |id: &serde_json::Value| match &doc.node(NodeId(id.as_u64().unwrap())).unwrap().kind {
        NodeKind::Path { path, .. } => path.bounds().unwrap(),
        _ => panic!("a path"),
    };
    let (front, side) = (bounds(&r["front"]), bounds(&r["side"]));
    for b in [front, side] {
        assert!((pt_to_mm(b.height()) - (1040.0 + 410.0)).abs() < 1e-6, "{b:?}");
    }
    // Front, as broad as the hips (an ellipse 960 round, 0.658 as deep as broad); from the side
    // as deep as the bust or the hips, whichever is deeper.
    let hip = pf_core::avatar::Ring { name: "hip", z: 0.0, x: 0.0, girth: 960.0, depth_ratio: 0.658 }.semi_axes();
    let bust = pf_core::avatar::Ring { name: "bust", z: 0.0, x: 0.0, girth: 880.0, depth_ratio: 0.918 }.semi_axes();
    assert!((pt_to_mm(front.width()) - 2.0 * hip.0).abs() < 1e-6, "{}", pt_to_mm(front.width()));
    assert!((pt_to_mm(side.width()) - 2.0 * bust.1.max(hip.1)).abs() < 1e-6, "{}", pt_to_mm(side.width()));
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
    // In metres, y up: the highest point is the base of the neck, 1.5 m or so up.
    let top = obj.lines().filter_map(|l| l.strip_prefix("v ")).filter_map(|l| l.split(' ').nth(1)?.parse::<f64>().ok()).fold(0.0, f64::max);
    assert!((top - (1040.0 + 63.0 * 410.0 / 416.0 + 410.0) / 1000.0).abs() < 1e-4, "{top}");
    let dir = std::env::temp_dir().join(format!("pf-avatar-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("me.obj");
    let r = s.execute("avatar.exportObj", &json!({"path": path.to_str().unwrap()})).unwrap();
    assert!(r.get("obj").is_none());
    assert_eq!(std::fs::read_to_string(&path).unwrap(), obj);
    std::fs::remove_dir_all(&dir).ok();
}
