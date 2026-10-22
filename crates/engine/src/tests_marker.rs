//! Marker making (PF-9, `marker.make`): every copy of a fabric's pieces laid on its width, grain
//! along the length, without overlapping.

use pf_core::{PatternTag, mm_to_pt, pt_to_mm};
use serde_json::{Value, json};
use vectorcraft_doc::{Node, NodeId, NodeKind};
use vectorcraft_geom::Rect;

use super::*;

/// A piece from a `w` × `h` mm rectangle at `x` mm, with its piece options `opts`.
fn rect_piece(s: &mut Session, x: f64, w: f64, h: f64, opts: Value) -> NodeId {
    let r = s.execute("shape.rectangle", &json!({"x": mm_to_pt(x), "y": 0, "width": mm_to_pt(w), "height": mm_to_pt(h)})).unwrap();
    let r = s.execute("piece.make", &json!({"ids": [r["id"]]})).unwrap();
    let id = NodeId(r["ids"][0].as_u64().unwrap());
    let mut o = opts;
    o["id"] = json!(id.0);
    s.execute("piece.set", &o).unwrap();
    id
}

fn layer<'a>(s: &'a Session, name: &str) -> &'a Node {
    s.doc().unwrap().doc.layers.iter().find(|l| l.name.as_deref() == Some(name)).unwrap()
}

/// The bounds of each laid copy's cutting line (its first path, unrotated rectangles here).
fn laid(l: &Node) -> Vec<(String, Rect)> {
    l.children()
        .unwrap()
        .iter()
        .filter(|g| matches!(g.kind, NodeKind::Group { .. }))
        .map(|g| {
            let cut = g.children().unwrap().iter().find(|c| c.name.as_deref() == Some("Cutting line")).unwrap();
            assert!(cut.extras().pattern.as_deref().and_then(PatternTag::mark).is_none(), "a copy is plain art, not a piece's part");
            for c in g.children().unwrap() {
                assert!(c.extras().draft.is_none() && c.extras().block.is_none(), "a copy is not drawn again from the measurements");
            }
            let NodeKind::Path { path, .. } = &cut.kind else { panic!("not a path") };
            (g.name.clone().unwrap_or_default(), path.bounds().unwrap())
        })
        .collect()
}

#[test]
fn a_marker_lays_every_copy_on_the_width_with_the_grain_along_the_length() {
    let mut s = Session::new();
    s.execute("file.new", &json!({"width": 2000, "height": 2000})).unwrap();
    // 300 × 500 mm with 10 mm allowance (320 × 520 cutting lines), grain down the piece: four
    // copies, two of them mirror images, and a lining piece.
    rect_piece(&mut s, 0.0, 300.0, 500.0, json!({"name": "Panel", "cut": 4, "mirror": true}));
    rect_piece(&mut s, 400.0, 200.0, 200.0, json!({"name": "Lining", "fabric": "Lining"}));
    let r = s.execute("marker.make", &json!({"width": "110 cm", "gap": 0, "origin": [0, 2000]})).unwrap();
    assert_eq!(r["fabric"], "Main");
    assert_eq!(r["copies"], 4);
    // Turned with the grain along the length, each is 520 long and 320 across: three across the
    // 1100 width, the fourth after them.
    let length = r["lengthMm"].as_f64().unwrap();
    assert!((length - 1040.0).abs() <= 5.0, "{r}");
    let e = r["efficiency"].as_f64().unwrap();
    assert!((e - 4.0 * 320.0 * 520.0 / (1100.0 * length) * 100.0).abs() < 0.2, "{r}");
    let l = layer(&s, "Marker Main");
    let copies = laid(l);
    assert_eq!(copies.len(), 4);
    assert_eq!(copies.iter().filter(|(n, _)| n == "Panel (mirrored)").count(), 2);
    // The fabric is true to scale; every copy lies on it, the long side along the length, and
    // none overlaps another.
    let fabric = l.children().unwrap().iter().find(|c| c.name.as_deref() == Some("Fabric Main")).unwrap();
    let NodeKind::Path { path, .. } = &fabric.kind else { panic!() };
    let f = path.bounds().unwrap();
    assert!((f.height() - mm_to_pt(1100.0)).abs() < 1e-6 && (pt_to_mm(f.width()) - length).abs() < 0.1, "{f:?}");
    for (i, (_, b)) in copies.iter().enumerate() {
        assert!((pt_to_mm(b.width()) - 520.0).abs() < 0.01 && (pt_to_mm(b.height()) - 320.0).abs() < 0.01, "{b:?}");
        assert!(b.x0 >= f.x0 - 1e-6 && b.x1 <= f.x1 + 1e-6 && b.y0 >= f.y0 - 1e-6 && b.y1 <= f.y1 + 1e-6);
        for (_, c) in &copies[i + 1..] {
            let o = b.intersect(*c);
            assert!(o.width() <= 1e-6 || o.height() <= 1e-6, "{b:?} overlaps {c:?}");
        }
    }
    // Made again it replaces the layer; the lining's marker is its own.
    let layers = s.doc().unwrap().doc.layers.len();
    s.execute("marker.make", &json!({"width": 1100, "gap": 0})).unwrap();
    assert_eq!(s.doc().unwrap().doc.layers.len(), layers);
    let r = s.execute("marker.make", &json!({"width": 1100, "fabric": "lining"})).unwrap();
    assert_eq!((r["copies"].as_u64(), r["fabric"].as_str()), (Some(1), Some("lining")));
    assert_eq!(laid(layer(&s, "Marker lining")).len(), 1);
    // Too narrow a fabric leaves a piece out and says so.
    let r = s.execute("marker.make", &json!({"width": 300})).unwrap();
    assert_eq!((r["copies"].as_u64(), r["notLaid"].clone()), (Some(0), json!(["Panel"])));
    assert!(s.execute("marker.make", &json!({})).is_err());
    assert!(s.execute("marker.make", &json!({"width": 1100, "fabric": "Silk"})).is_err());
    // Undo takes the marker away.
    s.execute("edit.undo", &json!({})).unwrap();
    assert!(s.doc().unwrap().doc.layers.iter().all(|l| l.name.as_deref() != Some("Marker Main") || laid(l).len() == 4));
}

#[test]
fn a_bodice_and_skirt_fit_on_two_metres() {
    let mut s = Session::new();
    s.execute("file.new", &json!({"width": 1400, "height": 1000})).unwrap();
    s.execute(
        "measure.set",
        &json!({"values": {"bust": 880, "waist": 680, "nape_to_waist": 410, "back_width": 345, "chest": 325, "shoulder": 122, "neck": 370, "armscye_depth": 210, "hip": 940, "waist_to_hip": 200, "skirt_length": 600}}),
    )
    .unwrap();
    let mut ids = Vec::new();
    for (block, origin) in [("block.bodice", [0.0, 0.0]), ("block.skirt", [0.0, 1500.0])] {
        let r = s.execute(block, &json!({"origin": origin})).unwrap();
        ids.extend(r["ids"].as_array().unwrap().iter().cloned());
    }
    let r = s.execute("piece.make", &json!({"ids": ids})).unwrap();
    for id in r["ids"].as_array().unwrap() {
        s.execute("piece.set", &json!({"id": id, "cut": 2, "mirror": true})).unwrap();
    }
    let r = s.execute("marker.make", &json!({"width": 1500, "oneWay": true})).unwrap();
    assert_eq!(r["copies"], 8, "{r}");
    assert!(r["lengthMm"].as_f64().unwrap() < 2000.0, "{r}");
    assert!(r["efficiency"].as_f64().unwrap() > 40.0, "{r}");
    // The marker's copies stay where they were laid when the measurements change.
    let before = layer(&s, "Marker Main").clone();
    s.execute("measure.set", &json!({"values": {"hip": 1000}})).unwrap();
    assert_eq!(layer(&s, "Marker Main"), &before);
}
