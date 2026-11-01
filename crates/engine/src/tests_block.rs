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

#[test]
fn the_sleeve_crown_is_checked_against_the_armhole() {
    let mut s = measured();
    s.execute("measure.set", &json!({"values": {"top_arm": 280, "sleeve_length": 590}})).unwrap();
    let bodice = s.execute("block.bodice", &json!({"origin": [0, 0]})).unwrap();
    let sleeve = s.execute("block.sleeve", &json!({"origin": [2000, 0]})).unwrap();
    let r = s.execute("design.sleeveEase", &json!({})).unwrap();
    let f = |k: &str| r[k].as_f64().unwrap();
    // Drafted where there is a bodice, the sleeve is set into its armhole with the middle of a
    // classic sleeve's ease (PF-4.9, the owner's decision).
    assert_eq!(r["fits"], "ok", "{r}");
    assert!((f("ease") - 44.45).abs() < 0.15, "{r}");
    // The crown is the sleeve block's own; the armhole the back's and the front's.
    assert!((f("crown") - sleeve["crown"].as_f64().unwrap()).abs() < 0.05, "{r}");
    assert!((f("armhole") - f("back") - f("front")).abs() < 0.15 && (f("ease") - (f("crown") - f("armhole"))).abs() < 0.15, "{r}");
    // The same seams named by hand measure the same.
    let (back, front, sl) = (bodice["ids"][0].clone(), bodice["ids"][1].clone(), sleeve["ids"][0].clone());
    let named = s
        .execute(
            "design.sleeveEase",
            &json!({"sleeve": {"id": sl, "from": 10, "to": 4}, "back": {"id": back, "from": 6, "to": 9}, "front": {"id": front, "from": 6, "to": 9}}),
        )
        .unwrap();
    assert_eq!(named["crown"], r["crown"]);
    assert_eq!(named["armhole"], r["armhole"]);
    // The sleeve_crown it gives redrafts the sleeve with the ease in the middle of the range
    // (1½ to 2 in for a classic sleeve).
    s.execute("measure.set", &json!({"values": {"sleeve_crown": r["sleeveCrown"]}})).unwrap();
    let r = s.execute("design.sleeveEase", &json!({})).unwrap();
    assert_eq!(r["fits"], "ok", "{r}");
    assert!((r["ease"].as_f64().unwrap() - 44.45).abs() < 0.15, "{r}");
    assert_eq!((r["min"].as_f64(), r["max"].as_f64()), (Some(38.1), Some(50.8)));
    // A shirt sleeve takes in less: the same crown is too full for one.
    let shirt = s.execute("design.sleeveEase", &json!({"style": "shirt"})).unwrap();
    assert_eq!(shirt["fits"], "full");
    assert!(shirt["advice"].as_str().unwrap().contains("pucker"), "{shirt}");
    let tight = s.execute("design.sleeveEase", &json!({"min": 60, "max": 70})).unwrap();
    assert_eq!(tight["fits"], "tight");
    assert!(s.execute("design.sleeveEase", &json!({"style": "dolman"})).is_err());
    assert!(s.execute("design.sleeveEase", &json!({"min": 50, "max": 40})).is_err());
    // A second sleeve block: which one is meant must be said.
    s.execute("block.sleeve", &json!({"origin": [3000, 0]})).unwrap();
    assert!(s.execute("design.sleeveEase", &json!({})).is_err());
}

#[test]
fn a_blocks_ease_and_darts_are_its_options_and_both_parts_follow() {
    let mut s = measured();
    let r = s.execute("block.bodice", &json!({"origin": [0, 0]})).unwrap();
    let (back, front) = (NodeId(r["ids"][0].as_u64().unwrap()), NodeId(r["ids"][1].as_u64().unwrap()));
    let info = s.execute("block.options", &json!({"id": front.0})).unwrap();
    assert_eq!(info["block"], "bodice");
    let ease = info["options"].as_array().unwrap().iter().find(|o| o["name"] == "ease").unwrap().clone();
    assert_eq!((ease["value"].clone(), ease["default"].clone()), (json!(null), json!(50.0)));
    // 7 cm ease: the front 2 cm wider; the back drawn with it too (its waist dart takes more in).
    let back_before = s.doc().unwrap().doc.node(back).unwrap().clone();
    s.execute("select.set", &json!({"ids": [front.0]})).unwrap();
    let r = s.execute("block.setOptions", &json!({"options": {"ease": 70, "shoulder_dart": "1.5 cm"}})).unwrap();
    assert_eq!(r["parts"].as_array().unwrap().len(), 2, "{r}");
    assert!(near(bounds(&s, front).x1, mm_to_pt(510.0)));
    assert_ne!(*s.doc().unwrap().doc.node(back).unwrap(), back_before);
    // They stay with the measurements: a bigger bust still grows it, with the ease.
    s.execute("measure.set", &json!({"values": {"bust": 960}})).unwrap();
    assert!(near(bounds(&s, front).x1, mm_to_pt(550.0)));
    // Bad options are refused and change nothing; null goes back to the default; undo works.
    assert!(s.execute("block.setOptions", &json!({"options": {"ease": 900}})).is_err());
    assert!(s.execute("block.setOptions", &json!({"options": {"hem": 5}})).is_err());
    s.execute("block.setOptions", &json!({"id": back.0, "options": {"ease": null}})).unwrap();
    assert!(near(bounds(&s, front).x1, mm_to_pt(530.0)));
    s.execute("edit.undo", &json!({})).unwrap();
    assert!(near(bounds(&s, front).x1, mm_to_pt(550.0)));
    // A piece made from the block: selected, its options are the block's.
    s.execute("select.set", &json!({"ids": [front.0]})).unwrap();
    s.execute("piece.make", &json!({})).unwrap();
    assert_eq!(s.execute("block.options", &json!({})).unwrap()["id"], json!(front.0));
    // Drafted with options from the start.
    s.execute("measure.set", &json!({"values": {"top_arm": 380, "sleeve_length": 640}})).unwrap();
    let r = s.execute("block.sleeve", &json!({"origin": [800, 0], "options": {"ease": 30}})).unwrap();
    let sleeve = r["ids"][0].as_u64().unwrap();
    let info = s.execute("block.options", &json!({"id": sleeve})).unwrap();
    assert_eq!(info["options"][0]["value"], json!(30.0));
    assert!(s.execute("block.skirt", &json!({"options": {"bust_dart": 10}})).is_err());
}

#[test]
fn the_trouser_seams_are_walked_front_against_back_run_by_run() {
    let mut s = measured();
    assert!(s.execute("design.trouserSeams", &json!({})).is_err(), "no trouser block drafted");
    s.execute(
        "measure.set",
        &json!({"values": {"hip": 960, "waist": 740, "waist_to_hip": 200, "crotch_depth": 260, "waist_to_knee": 600, "waist_to_ankle": 1040, "trouser_hem": 460}}),
    )
    .unwrap();
    s.execute("block.trousers", &json!({})).unwrap();
    let r = s.execute("design.trouserSeams", &json!({})).unwrap();
    let runs = r["runs"].as_array().unwrap();
    let seams: Vec<&str> = runs.iter().filter_map(|x| x["seam"].as_str()).collect();
    assert_eq!(seams, ["inseam, knee to hem", "inseam, crotch to knee", "side seam, knee to hem", "side seam, waist to knee"]);
    for run in runs {
        let (f, b, d) = (run["front"].as_f64().unwrap(), run["back"].as_f64().unwrap(), run["difference"].as_f64().unwrap());
        assert!(f > 0.0 && b > 0.0 && (b - f - d).abs() < 0.11, "{run}");
    }
    // Knee to hem is 44 cm down; the legs slant a little, front and back alike.
    for run in &runs[..1] {
        assert!((440.0..446.0).contains(&run["front"].as_f64().unwrap()), "{run}");
    }
    // The back inseam is more curved, so longer above the knee, as the tutorial expects.
    assert!(runs[1]["difference"].as_f64().unwrap() > 0.0, "{}", runs[1]);
    assert!(r["largest"].as_f64().unwrap() >= runs[1]["difference"].as_f64().unwrap());
    // With a 1 cm front dart the front waist is 1 cm wider at the side seam; the seams still walk.
    // The side seam's waist end moves out toward the side line, so it slants less: shorter.
    let ids: Vec<u64> = {
        let mut v = Vec::new();
        s.doc().unwrap().doc.walk(|n| {
            if n.extras().block.as_deref().is_some_and(|b| b.kind == pf_core::block::BlockKind::Trousers) {
                v.push(n.id.0);
            }
        });
        v
    };
    s.execute("block.setOptions", &json!({"id": ids[0], "options": {"front_dart": 10}})).unwrap();
    let with = s.execute("design.trouserSeams", &json!({})).unwrap();
    assert_eq!(with["runs"][0], runs[0], "below the knee nothing changes");
    assert!(with["runs"][3]["front"].as_f64().unwrap() < runs[3]["front"].as_f64().unwrap(), "{}", with["runs"][3]);
}

#[test]
fn the_trouser_waistband_is_half_the_waist_long_and_notched_at_the_side_seam() {
    let mut s = measured();
    assert!(s.execute("design.trouserWaistband", &json!({})).is_err(), "no trouser block drafted");
    s.execute(
        "measure.set",
        &json!({"values": {"hip": 960, "waist": 740, "waist_to_hip": 200, "crotch_depth": 260, "waist_to_knee": 600, "waist_to_ankle": 1040, "trouser_hem": 460}}),
    )
    .unwrap();
    s.execute("block.trousers", &json!({})).unwrap();
    let r = s.execute("design.trouserWaistband", &json!({})).unwrap();
    // Half the 74 cm waist, give or take the waist curves; 2 × 3 cm wide.
    let length = r["length"].as_f64().unwrap();
    assert!((365.0..380.0).contains(&length), "{r}");
    assert_eq!(r["width"], json!(60.0));
    let id = NodeId(r["id"].as_u64().unwrap());
    let info = s.execute("piece.info", &json!({"id": id.0})).unwrap();
    assert_eq!((info["name"].as_str(), info["cut"].as_u64(), info["mirror"].as_bool()), (Some("Waistband"), Some(2), Some(true)));
    let notches = info["notches"].as_array().unwrap();
    assert_eq!(notches.len(), 2);
    let at = notches[0]["at"].as_f64().unwrap();
    assert!((at - r["front"].as_f64().unwrap() / length).abs() < 1e-3, "{info}");
    // True scale: the seam line is the band, 2 × (length + 60) round.
    assert!((info["seamLength"].as_f64().unwrap() - 2.0 * (length + 60.0)).abs() < 0.2, "{info}");
    // One undo step.
    s.execute("edit.undo", &json!({})).unwrap();
    assert!(s.execute("piece.info", &json!({"id": id.0})).is_err());
    // With a front dart the waist is as long with the dart closed.
    let mut front = None;
    s.doc().unwrap().doc.walk(|n| {
        if n.extras().block.as_deref().is_some_and(|b| b.kind == pf_core::block::BlockKind::Trousers && b.part == 0) {
            front = Some(n.id.0);
        }
    });
    s.execute("block.setOptions", &json!({"id": front.unwrap(), "options": {"front_dart": 10}})).unwrap();
    let darted = s.execute("design.trouserWaistband", &json!({"height": 40})).unwrap();
    assert!((darted["length"].as_f64().unwrap() - length).abs() < 2.0, "{darted} {r}");
    assert_eq!(darted["width"], json!(80.0));
    assert!(s.execute("design.trouserWaistband", &json!({"height": 500})).is_err());
}

#[test]
fn a_sleeve_drafted_first_is_set_into_the_bodice_drafted_after_it_and_follows_it() {
    let mut s = measured();
    s.execute("measure.set", &json!({"values": {"top_arm": 280, "sleeve_length": 590}})).unwrap();
    // Alone, from the top arm table.
    let sleeve = s.execute("block.sleeve", &json!({"origin": [2000, 0]})).unwrap();
    let alone = sleeve["crown"].as_f64().unwrap();
    s.execute("block.bodice", &json!({"origin": [0, 0]})).unwrap();
    let r = s.execute("design.sleeveEase", &json!({})).unwrap();
    assert!((r["ease"].as_f64().unwrap() - 44.45).abs() < 0.15 && r["crown"].as_f64().unwrap() > alone, "{r} {alone}");
    // A bigger bust makes a bigger armhole: the sleeve's crown follows it.
    s.execute("measure.set", &json!({"values": {"bust": 960}})).unwrap();
    let r2 = s.execute("design.sleeveEase", &json!({})).unwrap();
    assert!(r2["armhole"].as_f64() != r["armhole"].as_f64() && (r2["ease"].as_f64().unwrap() - 44.45).abs() < 0.15, "{r2}");
    // The crown ease is the sleeve's option.
    let id = sleeve["ids"][0].as_u64().unwrap();
    s.execute("block.setOptions", &json!({"id": id, "options": {"crown_ease": 30}})).unwrap();
    let r3 = s.execute("design.sleeveEase", &json!({})).unwrap();
    assert!((r3["ease"].as_f64().unwrap() - 30.0).abs() < 0.15 && r3["fits"] == "tight", "{r3}");
}
