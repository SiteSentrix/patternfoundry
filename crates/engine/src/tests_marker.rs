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

/// The bounds of the paths named `name` in each group of layer `l`, by group name.
fn parts_named(l: &Node, name: &str) -> Vec<(String, Vec<Rect>)> {
    l.children()
        .unwrap()
        .iter()
        .filter(|g| matches!(g.kind, NodeKind::Group { .. }))
        .map(|g| {
            let b = g
                .children()
                .unwrap()
                .iter()
                .filter(|c| c.name.as_deref() == Some(name))
                .map(|c| match &c.kind {
                    NodeKind::Path { path, .. } => path.bounds().unwrap(),
                    _ => panic!("not a path"),
                })
                .collect();
            (g.name.clone().unwrap_or_default(), b)
        })
        .collect()
}

#[test]
fn a_piece_on_the_fold_opens_out_or_lies_against_the_fold() {
    let mut s = Session::new();
    s.execute("file.new", &json!({"width": 2000, "height": 2000})).unwrap();
    // A 300 × 500 half back, its longest straight edge (the left side, edge 3) on the fold: no
    // allowance there.
    let back = rect_piece(&mut s, 0.0, 300.0, 500.0, json!({"name": "Back", "fold": "auto"}));
    let info = s.execute("piece.info", &json!({"id": back.0})).unwrap();
    assert_eq!(info["fold"], 3);
    let cut =
        s.doc().unwrap().doc.node(back).unwrap().children().unwrap().iter().find(|c| c.name.as_deref() == Some("Cutting line")).cloned().unwrap();
    let NodeKind::Path { path, .. } = &cut.kind else { panic!() };
    let b = path.bounds().unwrap();
    assert!((pt_to_mm(b.width()) - 310.0).abs() < 0.01 && b.x0.abs() < 1e-6, "{b:?}");
    assert!(s.execute("piece.set", &json!({"id": back.0, "fold": 9})).is_err());
    // Laid single, it opens out whole: 520 along the length, 620 across, one cutting line.
    let r = s.execute("marker.make", &json!({"width": 1100, "gap": 0, "origin": [0, 2000]})).unwrap();
    assert_eq!(r["copies"], 1);
    assert!((r["lengthMm"].as_f64().unwrap() - 520.0).abs() < 0.01, "{r}");
    let l = layer(&s, "Marker Main");
    let cuts = parts_named(l, "Cutting line");
    assert_eq!(cuts.len(), 1);
    assert_eq!(cuts[0].1.len(), 1, "one cutting line round the whole");
    let w = cuts[0].1[0];
    assert!((pt_to_mm(w.width()) - 520.0).abs() < 0.01 && (pt_to_mm(w.height()) - 620.0).abs() < 0.01, "{w:?}");
    // The seam line and the grainline on both halves.
    let g = l.children().unwrap().iter().find(|g| g.name.as_deref() == Some("Back")).unwrap();
    assert_eq!(g.children().unwrap().iter().filter(|c| matches!(c.kind, NodeKind::Path { .. })).count(), 5);
    // Folded, with a 200 × 300 piece cut 2 (one pair through both layers) and one cut 3.
    rect_piece(&mut s, 400.0, 200.0, 300.0, json!({"name": "Sleeve", "cut": 2, "mirror": true}));
    rect_piece(&mut s, 700.0, 100.0, 100.0, json!({"name": "Tab", "cut": 3}));
    let r = s.execute("marker.make", &json!({"width": 1100, "gap": 0, "folded": true, "origin": [0, 2000]})).unwrap();
    assert_eq!(r["copies"], 4, "{r}");
    assert_eq!(r["warnings"], json!(["Tab: cut 3 on folded fabric cuts 4 (pairs)"]));
    let l = layer(&s, "Marker Main");
    let fabric = l.children().unwrap().iter().find(|c| c.name.as_deref() == Some("Fabric Main")).unwrap();
    let NodeKind::Path { path, .. } = &fabric.kind else { panic!() };
    let f = path.bounds().unwrap();
    assert!((pt_to_mm(f.height()) - 550.0).abs() < 1e-6, "half the width: {f:?}");
    assert!(l.children().unwrap().iter().any(|c| c.name.as_deref() == Some("Fold")));
    let cuts = parts_named(l, "Cutting line");
    let laid = cuts.iter().find(|(n, _)| n == "Back").unwrap().1[0];
    assert!((laid.y0 - f.y0).abs() < 1e-6 && (pt_to_mm(laid.height()) - 310.0).abs() < 0.01, "against the fold: {laid:?}");
    assert_eq!(cuts.iter().filter(|(n, _)| n == "Sleeve (pair)").count(), 1);
    assert_eq!(cuts.iter().filter(|(n, _)| n == "Tab (pair)").count(), 2);
    // No fold: back to a half with its allowance all round.
    s.execute("piece.set", &json!({"id": back.0, "fold": null})).unwrap();
    assert!(s.execute("piece.info", &json!({"id": back.0})).unwrap().get("fold").is_none());
}

/// Pieces moved by hand on a marker (PF-9.3): Check Marker draws the fabric to the last piece
/// again, brings the caption's length and % used up to date, and lists overlaps and pieces off
/// the fabric.
#[test]
fn a_marker_checked_after_moving_pieces_by_hand() {
    let mut s = Session::new();
    s.execute("file.new", &json!({"width": 2000, "height": 2000})).unwrap();
    rect_piece(&mut s, 0.0, 300.0, 500.0, json!({"name": "Panel", "cut": 2}));
    let r = s.execute("marker.make", &json!({"width": "110 cm", "gap": 0, "origin": [0, 2000]})).unwrap();
    let id = NodeId(r["layer"].as_u64().unwrap());
    let made = r["lengthMm"].as_f64().unwrap();
    // As laid: the same length, nothing wrong.
    let c = s.execute("marker.check", &json!({})).unwrap();
    assert!((c["lengthMm"].as_f64().unwrap() - made).abs() < 0.1, "{c} {r}");
    assert_eq!((c["overlaps"].clone(), c["offFabric"].clone()), (json!([]), json!([])));
    // The second copy moved 200 mm along: the fabric 200 longer, less of it used.
    let groups: Vec<NodeId> =
        layer(&s, "Marker Main").children().unwrap().iter().filter(|g| matches!(g.kind, NodeKind::Group { .. })).map(|g| g.id).collect();
    let far = *laid(layer(&s, "Marker Main")).iter().zip(&groups).max_by(|a, b| a.0.1.x1.total_cmp(&b.0.1.x1)).map(|(_, g)| g).unwrap();
    s.execute("select.set", &json!({"ids": [far.0]})).unwrap();
    s.execute("object.move", &json!({"dx": mm_to_pt(200.0)})).unwrap();
    let c = s.execute("marker.check", &json!({})).unwrap();
    assert!((c["lengthMm"].as_f64().unwrap() - (made + 200.0)).abs() < 0.1, "{c}");
    let fabric = layer(&s, "Marker Main").children().unwrap().iter().find(|n| n.name.as_deref() == Some("Fabric Main")).unwrap().clone();
    let NodeKind::Path { path, .. } = &fabric.kind else { panic!("a path") };
    assert!((pt_to_mm(path.bounds().unwrap().width()) - (made + 200.0)).abs() < 0.1);
    let caption = layer(&s, "Marker Main")
        .children()
        .unwrap()
        .iter()
        .find_map(|n| match &n.kind {
            NodeKind::Text(t) => Some(t.plain_text()),
            _ => None,
        })
        .unwrap();
    assert!(
        caption.starts_with("Main: 110 cm wide, ") && caption.contains(&format!("{} cm long", ((made + 200.0) / 10.0 * 10.0).round() / 10.0)),
        "{caption}"
    );
    // Back, and 100 mm across onto the other copy (they lie side by side across the width): they
    // overlap. 900 mm across: past the selvedge.
    let (a, b) = (laid(layer(&s, "Marker Main"))[0].1, laid(layer(&s, "Marker Main"))[1].1);
    let (moved, other) = if far == groups[0] { (a, b) } else { (b, a) };
    let toward = moved.y0 - other.y0;
    s.execute("object.move", &json!({"dx": -mm_to_pt(200.0), "dy": other.y0 + mm_to_pt(100.0) * toward.signum() - moved.y0})).unwrap();
    let c = s.execute("marker.check", &json!({"layer": id.0})).unwrap();
    assert_eq!(c["overlaps"].as_array().unwrap().len(), 1, "{c}");
    assert!((c["lengthMm"].as_f64().unwrap() - made).abs() < 0.1, "{c}");
    s.execute("object.move", &json!({"dy": toward.signum() * mm_to_pt(1000.0)})).unwrap();
    let c = s.execute("marker.check", &json!({})).unwrap();
    assert_eq!(c["offFabric"].as_array().unwrap().len(), 1, "{c}");
    assert!(s.execute("marker.check", &json!({"layer": 1})).is_err(), "not a marker's layer");
}
