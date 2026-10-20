//! Blocks (PF-4, `block.*`): the bodice, skirt, sleeve and trouser blocks draft from the measurements and follow them.

use pf_core::mm_to_pt;
use serde_json::json;
use vectorcraft_doc::{NodeId, NodeKind};
use vectorcraft_geom::Rect;

use super::*;

/// Made-up measurements (not from any size chart), in mm.
fn measured() -> Session {
    let mut s = Session::new();
    s.execute("file.new", &json!({"width": 1400, "height": 1000})).unwrap();
    s.execute(
        "measure.set",
        &json!({"values": {"bust": 880, "waist": 680, "nape_to_waist": 410, "back_width": 345, "chest": 325, "shoulder": 122, "neck": 370, "armscye_depth": 210}}),
    )
    .unwrap();
    s
}

fn bounds(s: &Session, id: NodeId) -> Rect {
    match &s.doc().unwrap().doc.node(id).unwrap().kind {
        NodeKind::Path { path, .. } => path.bounds().unwrap(),
        _ => panic!("not a path"),
    }
}

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-6
}

#[test]
fn the_bodice_block_drafts_from_the_measurements_and_follows_the_bust() {
    let mut s = measured();
    let r = s.execute("block.bodice", &json!({"origin": [0, 0]})).unwrap();
    let ids: Vec<NodeId> = r["ids"].as_array().unwrap().iter().map(|v| NodeId(v.as_u64().unwrap())).collect();
    assert_eq!(ids.len(), 2);
    let names: Vec<Option<String>> = ids.iter().map(|id| s.doc().unwrap().doc.node(*id).unwrap().name.clone()).collect();
    assert_eq!(names, [Some("Back Bodice Block".into()), Some("Front Bodice Block".into())]);
    // Centre back at 0, centre front at half bust plus 5 cm ease; the back is nape to waist long.
    let (back, front) = (bounds(&s, ids[0]), bounds(&s, ids[1]));
    assert!(near(back.x0, 0.0) && near(back.y1, mm_to_pt(410.0)), "{back:?}");
    assert!(near(front.x1, mm_to_pt(490.0)), "{front:?}");
    // A bigger bust: the front grows, in one undo step.
    s.execute("measure.set", &json!({"values": {"bust": 960}})).unwrap();
    assert!(near(bounds(&s, ids[1]).x1, mm_to_pt(530.0)));
    s.execute("edit.undo", &json!({})).unwrap();
    assert!(near(bounds(&s, ids[1]).x1, mm_to_pt(490.0)));
}

#[test]
fn a_block_made_a_pattern_piece_keeps_following_and_its_cutting_line_with_it() {
    let mut s = measured();
    let r = s.execute("block.bodice", &json!({"origin": [0, 0]})).unwrap();
    let front = NodeId(r["ids"][1].as_u64().unwrap());
    s.execute("select.set", &json!({"ids": [front.0]})).unwrap();
    let g = NodeId(s.execute("piece.make", &json!({"name": "Front"})).unwrap()["ids"][0].as_u64().unwrap());
    let cut_right = |s: &Session| {
        let group = s.doc().unwrap().doc.node(g).unwrap().clone();
        let cut = group
            .children()
            .unwrap()
            .iter()
            .find(|c| c.extras().pattern.as_deref().and_then(pf_core::PatternTag::mark) == Some(pf_core::Mark::Cut))
            .unwrap()
            .clone();
        let NodeKind::Path { path, .. } = &cut.kind else { panic!() };
        path.bounds().unwrap().x1
    };
    assert!(near(cut_right(&s), mm_to_pt(500.0)), "{}", cut_right(&s));
    s.execute("measure.set", &json!({"values": {"bust": 960}})).unwrap();
    assert!(near(bounds(&s, front).x1, mm_to_pt(530.0)));
    assert!(near(cut_right(&s), mm_to_pt(540.0)));
}

#[test]
fn moving_a_block_keeps_it_where_it_was_moved() {
    let mut s = measured();
    let r = s.execute("block.bodice", &json!({"origin": [0, 0]})).unwrap();
    let back = NodeId(r["ids"][0].as_u64().unwrap());
    s.execute("select.set", &json!({"ids": [back.0]})).unwrap();
    s.execute("object.move", &json!({"dx": 30, "dy": 10})).unwrap();
    assert!(near(bounds(&s, back).x0, 30.0));
    // A measurement changes: the moved back is drawn again where it now is.
    s.execute("measure.set", &json!({"values": {"nape_to_waist": 420}})).unwrap();
    let b = bounds(&s, back);
    assert!(near(b.x0, 30.0) && near(b.y1, 10.0 + mm_to_pt(420.0)), "{b:?}");
}

#[test]
fn missing_measurements_are_named() {
    let mut s = Session::new();
    s.execute("file.new", &json!({})).unwrap();
    s.execute("measure.set", &json!({"values": {"bust": 900, "waist": 700}})).unwrap();
    let e = s.execute("block.bodice", &json!({})).unwrap_err().to_string();
    assert!(e.contains("nape_to_waist") && e.contains("armscye_depth") && !e.contains("bust,"), "{e}");
    let m = s.execute("block.measurements", &json!({})).unwrap();
    assert_eq!(m["missing"], json!(["nape_to_waist", "back_width", "chest", "shoulder", "neck", "armscye_depth"]));
    assert_eq!(m["needed"].as_array().unwrap().len(), 8);
    assert!(s.execute("block.bodice", &json!({"origin": [1]})).is_err());
}

#[test]
fn the_skirt_block_drafts_from_its_measurements_and_follows_the_hip() {
    let mut s = Session::new();
    s.execute("file.new", &json!({"width": 1400, "height": 1000})).unwrap();
    let m = s.execute("block.measurements", &json!({"block": "skirt"})).unwrap();
    assert_eq!(m["missing"], json!(["waist", "hip", "waist_to_hip", "skirt_length"]));
    assert!(s.execute("block.skirt", &json!({})).is_err());
    s.execute("measure.set", &json!({"values": {"waist": 680, "hip": 940, "waist_to_hip": 200, "skirt_length": 600}})).unwrap();
    let r = s.execute("block.skirt", &json!({"origin": [0, 0]})).unwrap();
    let ids: Vec<NodeId> = r["ids"].as_array().unwrap().iter().map(|v| NodeId(v.as_u64().unwrap())).collect();
    assert_eq!(s.doc().unwrap().doc.node(ids[1]).unwrap().name.as_deref(), Some("Front Skirt Block"));
    // Centre back at 0 and skirt length long; centre front at half the hip plus 1 cm ease.
    let (back, front) = (bounds(&s, ids[0]), bounds(&s, ids[1]));
    assert!(near(back.x0, 0.0) && near(back.y1, mm_to_pt(600.0)), "{back:?}");
    assert!(near(front.x1, mm_to_pt(480.0)), "{front:?}");
    s.execute("measure.set", &json!({"values": {"hip": 1000}})).unwrap();
    assert!(near(bounds(&s, ids[1]).x1, mm_to_pt(510.0)));
    assert!(s.execute("block.measurements", &json!({"block": "trouser"})).is_err());
}

#[test]
fn the_sleeve_block_drafts_inside_the_artboard_and_follows_the_top_arm() {
    let mut s = measured();
    s.execute("measure.set", &json!({"values": {"top_arm": 280, "sleeve_length": 590}})).unwrap();
    let r = s.execute("block.sleeve", &json!({})).unwrap();
    let id = NodeId(r["ids"][0].as_u64().unwrap());
    assert_eq!(s.doc().unwrap().doc.node(id).unwrap().name.as_deref(), Some("Sleeve Block"));
    // By default the sleeve itself starts 20 mm inside the artboard (its shoulder point, the
    // origin, is half a sleeve further in).
    let art = s.doc().unwrap().doc.artboards[0].rect;
    let b = bounds(&s, id);
    assert!(near(b.x0, art.x0 + mm_to_pt(20.0)) && near(b.y0, art.y0 + mm_to_pt(20.0)), "{b:?}");
    assert!(near(b.width(), mm_to_pt(330.0)), "{}", b.width());
    // The crown is longer than the bodice's armhole (back and front), as a set-in sleeve's must
    // be; its ease is reported, not forced (truing it is design.matchLength's).
    let crown = r["crown"].as_f64().unwrap();
    let bodice = s.execute("block.bodice", &json!({"origin": [0, 0]})).unwrap();
    let armhole: f64 = (0..2)
        .map(|k| {
            let part = bodice["ids"][k].as_u64().unwrap();
            s.execute("design.length", &json!({"id": part, "from": 6, "to": 9})).unwrap()["mm"].as_f64().unwrap()
        })
        .sum();
    // By the table alone the crown is about the armhole's length (here 1.6 mm short), so the
    // sleeve is fitted: the armhole as a measurement, the crown 3 cm more.
    assert!((crown - armhole).abs() < 20.0, "crown {crown}, armhole {armhole}");
    s.execute("measure.set", &json!({"values": {"armhole": armhole}, "formulas": {"sleeve_crown": "armhole + 30"}})).unwrap();
    let fitted = s.execute("block.sleeve", &json!({})).unwrap();
    assert!((fitted["crown"].as_f64().unwrap() - (armhole + 30.0)).abs() < 1e-6, "{fitted}");
    // The first sleeve followed too: it is the same shape as the new one.
    let other = NodeId(fitted["ids"][0].as_u64().unwrap());
    assert!(near(bounds(&s, id).height(), bounds(&s, other).height()));
    s.execute("measure.set", &json!({"formulas": {"sleeve_crown": null}, "values": {"armhole": null}})).unwrap();
    // A larger top arm: a wider sleeve, in one undo step.
    s.execute("measure.set", &json!({"values": {"top_arm": 320}})).unwrap();
    assert!(near(bounds(&s, id).width(), mm_to_pt(370.0)));
    s.execute("edit.undo", &json!({})).unwrap();
    assert!(near(bounds(&s, id).width(), mm_to_pt(330.0)));
    let m = s.execute("block.measurements", &json!({"block": "sleeve"})).unwrap();
    assert_eq!(m["missing"], json!([]));
    s.execute("measure.set", &json!({"values": {"sleeve_length": null}})).unwrap();
    let m = s.execute("block.measurements", &json!({"block": "sleeve"})).unwrap();
    assert_eq!(m["missing"], json!(["sleeve_length"]));
    assert!(s.execute("block.sleeve", &json!({})).is_err());
}

#[test]
fn the_trouser_block_drafts_inside_the_artboard_and_follows_the_hip() {
    let mut s = measured();
    s.execute(
        "measure.set",
        &json!({"values": {"hip": 960, "waist": 740, "waist_to_hip": 200, "crotch_depth": 260, "waist_to_knee": 600, "waist_to_ankle": 1040, "trouser_hem": 460}}),
    )
    .unwrap();
    let r = s.execute("block.trousers", &json!({})).unwrap();
    let ids: Vec<NodeId> = r["ids"].as_array().unwrap().iter().map(|v| NodeId(v.as_u64().unwrap())).collect();
    let names: Vec<Option<String>> = ids.iter().map(|id| s.doc().unwrap().doc.node(*id).unwrap().name.clone()).collect();
    assert_eq!(names, [Some("Front Trouser Block".into()), Some("Back Trouser Block".into())]);
    // The front's fork reaches left of the frame and the back's waist above it: the block is
    // still drawn 20 mm inside the artboard.
    let art = s.doc().unwrap().doc.artboards[0].rect;
    let (f, b) = (bounds(&s, ids[0]), bounds(&s, ids[1]));
    assert!(near(f.x0, art.x0 + mm_to_pt(20.0)) && near(b.y0.min(f.y0), art.y0 + mm_to_pt(20.0)), "{f:?} {b:?}");
    // From the fork to the side seam, 6 cm + 25 cm across at the crotch line; the leg 104 cm.
    assert!(near(f.height(), mm_to_pt(1040.0)), "{}", f.height());
    let width = f.width();
    s.execute("measure.set", &json!({"values": {"hip": 1040}})).unwrap();
    assert!(bounds(&s, ids[0]).width() > width);
    s.execute("edit.undo", &json!({})).unwrap();
    assert!(near(bounds(&s, ids[0]).width(), width));
    let m = s.execute("block.measurements", &json!({"block": "trousers"})).unwrap();
    assert_eq!(m["missing"], json!([]));
}
