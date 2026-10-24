//! Draping (PF-10.3, `drape.make`): pieces sewn together wrapped round the avatar and let fall
//! onto it as cloth.

use pf_core::mm_to_pt;
use serde_json::json;

use super::*;

/// An A-line skirt of two panels (front and back), each 360 mm at the waist and 800 at the hem,
/// 600 long, sewn at both sides, for a body 680 round the waist and 960 round the hips: 720 round
/// at its waist, 1013 at the hip line 200 mm down (5 cm of ease).
fn skirt() -> Session {
    let mut s = Session::new();
    s.execute("file.new", &json!({"width": 2000, "height": 1000})).unwrap();
    s.execute("measure.set", &json!({"values": {"bust": 880, "waist": 680, "hip": 960, "nape_to_waist": 410, "waist_to_hip": 200}})).unwrap();
    let mut ids = Vec::new();
    for (name, x) in [("Front", 0.0), ("Back", 900.0)] {
        let m = |v: f64| mm_to_pt(v);
        let d = format!("M{} 0 L{} 0 L{} {} L{} {} Z", m(x + 220.0), m(x + 580.0), m(x + 800.0), m(600.0), m(x), m(600.0));
        let path = s.execute("path.create", &json!({"d": d})).unwrap()["id"].clone();
        ids.push(s.execute("piece.make", &json!({"ids": [path], "name": name})).unwrap()["ids"][0].clone());
    }
    // Anchors 0 the waist's left end, 1 its right, 2 the hem's right, 3 its left: each side seam
    // runs down one panel's side and up the other's.
    s.execute("piece.sew", &json!({"a": {"id": ids[0], "from": 1, "to": 2}, "b": {"id": ids[1], "from": 3, "to": 0}})).unwrap();
    s.execute("piece.sew", &json!({"a": {"id": ids[1], "from": 1, "to": 2}, "b": {"id": ids[0], "from": 3, "to": 0}})).unwrap();
    s
}

#[test]
fn a_skirt_drapes_on_the_avatar_resting_at_the_waist() {
    let mut s = skirt();
    let r = s.execute("drape.make", &json!({"precision": 20, "steps": 120})).unwrap();
    assert_eq!(r["pieces"], json!(2), "{r}");
    let (top, hem, waist) = (r["top"].as_f64().unwrap(), r["hem"].as_f64().unwrap(), r["waist"].as_f64().unwrap());
    // 720 round at its waist, it settles a little below the body's 680 waist, where the body is
    // as big round, and hangs its 600 mm from there (it doesn't slide off), a few per cent longer
    // under its own weight; its side seams closed, its edges on the seam lines.
    assert!(top < waist + 10.0 && top > waist - 60.0, "{r}");
    assert!((580.0..680.0).contains(&(top - hem)), "{r}");
    assert!(r["widestSeam"].as_f64().unwrap() < 5.0, "{r}");
    assert!(r["worstStretch"].as_f64().unwrap() < 15.0, "{r}");
    // The draped cloth as a mesh.
    let dir = std::env::temp_dir().join(format!("pf-drape-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("skirt.obj");
    let r = s.execute("drape.make", &json!({"steps": 10, "path": path.to_str().unwrap()})).unwrap();
    let obj = std::fs::read_to_string(&path).unwrap();
    assert_eq!(obj.lines().filter(|l| l.starts_with("v ")).count(), r["particles"].as_u64().unwrap() as usize);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn draping_needs_sewn_pieces_and_the_measurements() {
    let mut s = Session::new();
    s.execute("file.new", &json!({"width": 800, "height": 800})).unwrap();
    assert!(s.execute("drape.make", &json!({})).unwrap_err().to_string().contains("bust"));
    s.execute("measure.set", &json!({"values": {"bust": 880, "waist": 680, "hip": 960, "nape_to_waist": 410, "waist_to_hip": 200}})).unwrap();
    assert!(s.execute("drape.make", &json!({})).unwrap_err().to_string().contains("sewn"));
    let mut s = skirt();
    assert!(s.execute("drape.make", &json!({"precision": 1})).is_err());
}
