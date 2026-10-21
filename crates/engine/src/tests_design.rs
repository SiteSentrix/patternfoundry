//! Design tools (PF-5, `design.*`): a princess-seam bodice from the bodice block, by pivoting the
//! bust dart into the armhole and splitting through the bust point; slash and spread on the skirt.

use pf_core::mm_to_pt;
use serde_json::json;
use vectorcraft_doc::{NodeId, NodeKind};
use vectorcraft_geom::{Point, Rect};

use super::*;

/// Made-up measurements (not from any size chart), in mm, and the front bodice block.
fn front() -> (Session, NodeId) {
    let mut s = Session::new();
    s.execute("file.new", &json!({"width": 1400, "height": 1000})).unwrap();
    s.execute(
        "measure.set",
        &json!({"values": {"bust": 880, "waist": 680, "nape_to_waist": 410, "back_width": 345, "chest": 325, "shoulder": 122, "neck": 370, "armscye_depth": 210}}),
    )
    .unwrap();
    let r = s.execute("block.bodice", &json!({"origin": [0, 0]})).unwrap();
    let id = NodeId(r["ids"][1].as_u64().unwrap());
    (s, id)
}

fn points(s: &Session, id: NodeId) -> Vec<Point> {
    match &s.doc().unwrap().doc.node(id).unwrap().kind {
        NodeKind::Path { path, .. } => path.subpaths[0].anchors.iter().map(|a| a.p).collect(),
        _ => panic!("not a path"),
    }
}

fn bounds(s: &Session, id: NodeId) -> Rect {
    match &s.doc().unwrap().doc.node(id).unwrap().kind {
        NodeKind::Path { path, .. } => path.bounds().unwrap(),
        _ => panic!("not a path"),
    }
}

fn length(s: &mut Session, id: NodeId, from: usize, to: usize) -> f64 {
    s.execute("design.length", &json!({"id": id.0, "from": from, "to": to})).unwrap()["mm"].as_f64().unwrap()
}

// The front block's points: 3 is the waist dart's point, 8 the armhole's balance point Y, 9 the
// shoulder point, 10 and 12 the bust dart's legs either side of its point, 11.

#[test]
fn a_princess_bodice_from_the_block() {
    let (mut s, id) = front();
    let before = points(&s, id);
    let bust_point = before[11];
    assert_eq!(before[3], bust_point);
    // Pivot the whole bust dart into the armhole at Y: the shoulder side swings, the centre front
    // stays.
    let r = s.execute("design.rotateDart", &json!({"id": id.0, "apex": 11, "toAnchor": 8})).unwrap();
    let apex = r["apex"].as_u64().unwrap() as usize;
    let after = points(&s, id);
    assert_eq!(after.len(), 13);
    assert_eq!(after[apex], bust_point);
    assert!(after.contains(&before[0]) && after.contains(&before[1]) && after.contains(&before[12]));
    // The shoulder dart is closed: the swung shoulder meets the neck point. The block's dart legs
    // differ by a few mm, which the shoulder takes up: it is a shoulder long within that.
    assert_eq!(after[0], before[12]);
    let legs = ((before[10] - bust_point).hypot() - (before[12] - bust_point).hypot()).abs();
    let shoulder = length(&mut s, id, 12, 0);
    assert!((shoulder - 122.0).abs() < pf_core::pt_to_mm(legs) && (shoulder - 122.0).abs() > 0.0, "{shoulder}");
    // The armhole dart's legs are as long.
    let legs = ((after[apex - 1] - bust_point).hypot(), (after[apex + 1] - bust_point).hypot());
    assert!((legs.0 - legs.1).abs() < 1e-9);
    // It no longer follows the measurements.
    let was = bounds(&s, id);
    s.execute("measure.set", &json!({"values": {"bust": 960}})).unwrap();
    assert_eq!(bounds(&s, id), was);
    // Split through the bust point the two darts share: the centre front and side front panels.
    let r = s.execute("design.split", &json!({"id": id.0, "anchors": [apex]})).unwrap();
    let ids: Vec<NodeId> = r["ids"].as_array().unwrap().iter().map(|v| NodeId(v.as_u64().unwrap())).collect();
    assert_eq!(ids.len(), 2);
    assert!(s.doc().unwrap().doc.node(id).is_none());
    let names: Vec<Option<String>> = ids.iter().map(|i| s.doc().unwrap().doc.node(*i).unwrap().name.clone()).collect();
    assert_eq!(names, [Some("Front Bodice Block 1".into()), Some("Front Bodice Block 2".into())]);
    let (cf, side) = (points(&s, ids[0]), points(&s, ids[1]));
    assert_eq!((cf.len(), side.len()), (7, 6));
    assert!(cf[0] == bust_point && side[0] == bust_point);
    // Together they are the front: the centre front panel holds the centre front, the side panel
    // the side seam.
    assert!(bounds(&s, ids[0]).x1 > bounds(&s, ids[1]).x1);
    // The princess seam on each panel: waist dart leg, bust point, armhole dart leg. They sew
    // together within the front waist's slope.
    let (a, b) = (length(&mut s, ids[0], 6, 1), length(&mut s, ids[1], 5, 1));
    assert!((a - b).abs() < 2.0 && (a - b).abs() > 1e-3, "{a} {b}");
    // True the side front's seam to the centre front's: its waist end moves down the seam.
    let r = s.execute("design.matchLength", &json!({"id": ids[1].0, "from": 5, "to": 1, "likeId": ids[0].0, "likeFrom": 6, "likeTo": 1})).unwrap();
    assert!((length(&mut s, ids[1], 5, 1) - a).abs() < 1e-6 && (r["moved"].as_f64().unwrap() - (a - b)).abs() < 0.1, "{r}");
    assert!((length(&mut s, ids[0], 6, 1) - a).abs() < 1e-9);
    let r = s.execute("design.matchLength", &json!({"id": ids[1].0, "from": 5, "to": 1, "length": 300})).unwrap();
    assert!((r["mm"].as_f64().unwrap() - 300.0).abs() < 1e-6);
    for p in [
        json!({"id": ids[1].0, "from": 5, "to": 1}),
        json!({"id": ids[1].0, "from": 5, "to": 5, "length": 10}),
        json!({"id": ids[1].0, "from": 5, "to": 1, "length": -3}),
        json!({"id": ids[1].0, "from": 5, "to": 1, "likeId": ids[0].0, "likeFrom": 6}),
    ] {
        assert!(s.execute("design.matchLength", &p).is_err(), "{p}");
    }
    s.execute("edit.undo", &json!({})).unwrap();
    s.execute("edit.undo", &json!({})).unwrap();
    // One undo step each.
    s.execute("edit.undo", &json!({})).unwrap();
    assert_eq!(points(&s, id), after);
    s.execute("edit.undo", &json!({})).unwrap();
    s.execute("edit.undo", &json!({})).unwrap();
    assert_eq!(points(&s, id), before);
}

#[test]
fn the_two_selected_points_say_which_dart_moves_where() {
    let (mut s, id) = front();
    s.execute("select.anchors", &json!({"id": id.0, "anchors": [[0, 8], [0, 11]]})).unwrap();
    let r = s.execute("design.rotateDart", &json!({})).unwrap();
    let (mut t, id2) = front();
    let r2 = t.execute("design.rotateDart", &json!({"id": id2.0, "apex": 11, "toAnchor": 8})).unwrap();
    assert_eq!(r["apex"], r2["apex"]);
    assert_eq!(points(&s, id), points(&t, id2));
}

#[test]
fn a_dart_moves_to_any_point_on_the_outline_and_part_of_it_can_stay() {
    let (mut s, id) = front();
    let before = points(&s, id);
    // Half the bust dart to the middle of the shoulder: a new point there, two darts open.
    let mid = Point::new((before[9].x + before[10].x) / 2.0, (before[9].y + before[10].y) / 2.0 + 0.5);
    let r = s.execute("design.rotateDart", &json!({"id": id.0, "apex": 11, "to": [mid.x, mid.y], "amount": 0.5})).unwrap();
    assert_eq!(r["anchors"], 16);
    let after = points(&s, id);
    assert_eq!(after.iter().filter(|p| **p == before[11]).count(), 3);
    // Points off the outline, on the dart, past the end, a bad amount or side: refused, unchanged.
    for p in [
        json!({"id": id.0, "apex": 11, "to": [-500, -500]}),
        json!({"id": id.0, "apex": 11, "toAnchor": 12}),
        json!({"id": id.0, "apex": 99, "toAnchor": 8}),
        json!({"id": id.0, "apex": 11, "toAnchor": 8, "amount": 2}),
        json!({"id": id.0, "apex": 11, "toAnchor": 8, "side": "up"}),
        json!({"id": id.0, "apex": 11}),
    ] {
        assert!(s.execute("design.rotateDart", &p).is_err(), "{p}");
    }
    assert_eq!(points(&s, id), after);
}

#[test]
fn a_piece_seam_line_takes_a_dart_but_splits_only_released() {
    let (mut s, id) = front();
    s.execute("select.set", &json!({"ids": [id.0]})).unwrap();
    let g = NodeId(s.execute("piece.make", &json!({"name": "Front"})).unwrap()["ids"][0].as_u64().unwrap());
    let r = s.execute("design.rotateDart", &json!({"id": g.0, "apex": 11, "toAnchor": 8})).unwrap();
    assert_eq!(r["id"], id.0);
    let apex = r["apex"].as_u64().unwrap();
    let e = s.execute("design.split", &json!({"id": g.0, "anchors": [apex]})).unwrap_err().to_string();
    assert!(e.contains("release"), "{e}");
    // A split needs two points, or one two darts share.
    let (mut t, id2) = front();
    assert!(t.execute("design.split", &json!({"id": id2.0, "anchors": [5]})).is_err());
    assert!(t.execute("design.split", &json!({"id": id2.0, "anchors": [5, 6]})).is_err());
    assert!(t.execute("design.split", &json!({"id": id2.0})).is_err());
    assert!(t.execute("design.length", &json!({"id": id2.0, "from": 0, "to": 40})).is_err());
}

/// The skirt block's front, and its points.
fn skirt_front() -> (Session, NodeId, Vec<Point>) {
    let mut s = Session::new();
    s.execute("file.new", &json!({"width": 1400, "height": 1000})).unwrap();
    s.execute("measure.set", &json!({"values": {"waist": 680, "hip": 940, "waist_to_hip": 200, "skirt_length": 600}})).unwrap();
    let r = s.execute("block.skirt", &json!({"origin": [0, 0]})).unwrap();
    let id = NodeId(r["ids"][1].as_u64().unwrap());
    let p = points(&s, id);
    (s, id, p)
}

#[test]
fn slash_and_spread_flares_the_skirt_at_the_hem() {
    // The front's points: 0 the centre front at the waist, 1 at the hem, 2 the side at the hem,
    // 7 the front dart's leg nearest the centre front. Slash from the hem straight up to the
    // waist half way between the dart and the centre front.
    let (mut s, id, before) = skirt_front();
    assert_eq!(before.len(), 8);
    let top = Point::new((before[7].x + before[0].x) / 2.0, (before[7].y + before[0].y) / 2.0);
    let hem = Point::new(top.x, before[1].y);
    let r = s.execute("design.spread", &json!({"id": id.0, "from": [hem.x, hem.y], "to": [top.x, top.y], "amount": 60})).unwrap();
    // Two new points on the slash, the one at the hem twice (either side of the gap).
    assert_eq!(r["anchors"], 11);
    let after = points(&s, id);
    assert!((after[0] - hem).hypot() < 1e-6);
    assert!(((after[10] - after[0]).hypot() - mm_to_pt(60.0)).abs() < 1e-9);
    // The waist end of the slash stays shut, the hem side seam doesn't move.
    assert!(after.iter().any(|p| (*p - top).hypot() < 1e-6) && after.contains(&before[2]));
    s.execute("edit.undo", &json!({})).unwrap();
    assert_eq!(points(&s, id), before);
    // Parallel: both ends open by 4 cm.
    let r = s.execute("design.spread", &json!({"id": id.0, "from": [hem.x, hem.y], "to": [top.x, top.y], "amount": 40, "mode": "parallel"})).unwrap();
    assert_eq!(r["anchors"], 12);
    let after = points(&s, id);
    assert!(((after[11] - after[0]).hypot() - mm_to_pt(40.0)).abs() < 1e-9);
    for p in [
        json!({"id": id.0, "from": 1, "to": 2, "amount": 10}),
        json!({"id": id.0, "from": [hem.x, hem.y], "to": [top.x, top.y]}),
        json!({"id": id.0, "from": [hem.x, hem.y], "to": [top.x, top.y], "amount": 10, "mode": "wide"}),
        json!({"id": id.0, "from": [-900, -900], "to": 0, "amount": 10}),
        json!({"id": id.0, "from": 99, "to": 0, "amount": 10}),
    ] {
        assert!(s.execute("design.spread", &p).is_err(), "{p}");
    }
    assert_eq!(points(&s, id), after);
}

#[test]
fn a_neck_facing_ends_on_the_shoulder_and_the_centre_front() {
    let (mut s, id) = front();
    s.execute("design.rotateDart", &json!({"id": id.0, "apex": 11, "toAnchor": 8})).unwrap();
    let body = points(&s, id);
    // The neck runs from the neck point (0) to the centre front (1); the shoulder ends at 0.
    let (neck, cf, shoulder) = (body[0], body[1], body[body.len() - 1]);
    let r = s.execute("design.facing", &json!({"id": id.0, "from": 0, "to": 1, "width": 50})).unwrap();
    let f = NodeId(r["id"].as_u64().unwrap());
    assert_eq!(s.doc().unwrap().doc.node(f).unwrap().name.as_deref(), Some("Front Bodice Block Facing"));
    let pts = points(&s, f);
    assert_eq!((pts[0], pts[1]), (neck, cf));
    // The inner end at the centre front is on the centre front (straight down from it), at least
    // the width below; the one at the shoulder on the shoulder line, past the neck point.
    let (cf_end, shoulder_end) = (pts[2], pts[pts.len() - 1]);
    assert!((cf_end.x - cf.x).abs() < 1e-6 && cf_end.y - cf.y >= mm_to_pt(50.0) - 1e-6, "{cf_end:?} {cf:?}");
    let (u, v) = (shoulder - neck, shoulder_end - neck);
    assert!((u.x * v.y - u.y * v.x).abs() / u.hypot() < 1e-6 && u.x * v.x + u.y * v.y > 0.0);
    // The body isn't changed; the facing is one undo step.
    assert_eq!(points(&s, id), body);
    s.execute("edit.undo", &json!({})).unwrap();
    assert!(s.doc().unwrap().doc.node(f).is_none());
    // Two selected points: the shorter way round, the same facing.
    s.execute("select.anchors", &json!({"id": id.0, "anchors": [[0, 1], [0, 0]]})).unwrap();
    let r = s.execute("design.facing", &json!({"width": 50})).unwrap();
    assert_eq!(points(&s, NodeId(r["id"].as_u64().unwrap())), pts);
    for p in [json!({"id": id.0, "from": 0, "to": 0}), json!({"id": id.0, "from": 0, "to": 1, "width": 0}), json!({"id": id.0, "from": 0})] {
        assert!(s.execute("design.facing", &p).is_err(), "{p}");
    }
}

#[test]
fn a_knife_pleat_in_a_skirt_piece() {
    let (mut s, id, before) = skirt_front();
    s.execute("select.set", &json!({"ids": [id.0]})).unwrap();
    let g = NodeId(s.execute("piece.make", &json!({"name": "Skirt Front"})).unwrap()["ids"][0].as_u64().unwrap());
    let top = Point::new((before[7].x + before[0].x) / 2.0, (before[7].y + before[0].y) / 2.0);
    let hem = Point::new(top.x, before[1].y);
    let r = s.execute("design.pleat", &json!({"id": g.0, "from": [hem.x, hem.y], "to": [top.x, top.y], "depth": 30})).unwrap();
    assert_eq!(r["id"], id.0);
    let after = points(&s, id);
    assert!(((after[after.len() - 1] - after[0]).hypot() - mm_to_pt(60.0)).abs() < 1e-9);
    // Both lines are in the piece: they move with it, and survive its redraws.
    let doc = &s.doc().unwrap().doc;
    let (fold, placement) = (NodeId(r["fold"].as_u64().unwrap()), NodeId(r["placement"].as_u64().unwrap()));
    assert_eq!((doc.parent_of(fold), doc.parent_of(placement)), (Some(g), Some(g)));
    match &doc.node(fold).unwrap().kind {
        NodeKind::Path { path, .. } => assert_eq!(path.subpaths.len(), 3),
        _ => panic!("not a path"),
    }
    assert!(doc.node(placement).unwrap().appearance.stroke().is_some_and(|st| st.dash.is_some()));
    assert!(doc.node(fold).unwrap().appearance.stroke().is_some_and(|st| st.dash.is_none()));
    s.execute("piece.set", &json!({"id": g.0, "allowance": 15})).unwrap();
    assert!(s.doc().unwrap().doc.node(fold).is_some());
    // One undo step: the outline back, the marks gone.
    s.execute("edit.undo", &json!({})).unwrap();
    s.execute("edit.undo", &json!({})).unwrap();
    assert_eq!(points(&s, id), before);
    assert!(s.doc().unwrap().doc.node(fold).is_none());
    assert!(s.execute("design.pleat", &json!({"id": g.0, "from": [hem.x, hem.y], "to": [top.x, top.y]})).is_err());
    assert!(s.execute("design.pleat", &json!({"id": g.0, "from": [hem.x, hem.y], "to": [top.x, top.y], "depth": 30, "kind": "accordion"})).is_err());
}

#[test]
fn an_inverted_pleat_has_two_fold_lines_meeting_at_its_middle() {
    let (mut s, id, before) = skirt_front();
    let top = Point::new((before[7].x + before[0].x) / 2.0, (before[7].y + before[0].y) / 2.0);
    let hem = Point::new(top.x, before[1].y);
    let r = s.execute("design.pleat", &json!({"id": id.0, "from": [hem.x, hem.y], "to": [top.x, top.y], "depth": 25, "kind": "inverted"})).unwrap();
    let after = points(&s, id);
    // Four times the depth: 10 cm more at the hem.
    assert!(((after[after.len() - 1] - after[0]).hypot() - mm_to_pt(100.0)).abs() < 1e-9);
    let doc = &s.doc().unwrap().doc;
    let (fold, placement) = (NodeId(r["fold"].as_u64().unwrap()), NodeId(r["placement"].as_u64().unwrap()));
    assert_eq!(doc.node(fold).unwrap().name.as_deref(), Some("Pleat fold lines"));
    // Two fold lines and two arrows (shaft and head each); one placement line in the middle.
    let subpaths = |id: NodeId| match &doc.node(id).unwrap().kind {
        NodeKind::Path { path, .. } => path.subpaths.clone(),
        _ => panic!("not a path"),
    };
    assert_eq!((subpaths(fold).len(), subpaths(placement).len()), (6, 1));
    let mid = subpaths(placement)[0].anchors[0].p;
    assert!(((mid - hem).hypot() - mm_to_pt(50.0)).abs() < 1e-6, "{mid:?}");
}

#[test]
fn a_gathering_mark_along_the_skirt_hem_says_what_it_gathers_into() {
    // The front's hem runs from the centre front (1) to the side seam (2): a quarter of the hip
    // with ease, 24 cm.
    let (mut s, id, before) = skirt_front();
    let r = s.execute("design.gather", &json!({"id": id.0, "from": 1, "to": 2, "into": 200})).unwrap();
    assert!((r["mm"].as_f64().unwrap() - 240.0).abs() < 1e-6, "{r}");
    assert!((r["ratio"].as_f64().unwrap() - 1.2).abs() < 1e-9);
    let mark = NodeId(r["mark"].as_u64().unwrap());
    let doc = &s.doc().unwrap().doc;
    assert_eq!(doc.node(mark).unwrap().name.as_deref(), Some("Gather to 200 mm"));
    match &doc.node(mark).unwrap().kind {
        NodeKind::Path { path, .. } => {
            assert_eq!(path.subpaths.len(), 3);
            // The wave keeps above the hem (inside the skirt), within 7 mm of it.
            let hem_y = before[1].y;
            assert!(path.subpaths[0].anchors.iter().all(|a| a.p.y < hem_y && hem_y - a.p.y <= mm_to_pt(7.0) + 1e-6));
        }
        _ => panic!("not a path"),
    }
    // The outline is unchanged; undo takes the mark away.
    assert_eq!(points(&s, id), before);
    s.execute("edit.undo", &json!({})).unwrap();
    assert!(s.doc().unwrap().doc.node(mark).is_none());
    for p in [json!({"id": id.0, "from": 1, "to": 2, "into": 300}), json!({"id": id.0, "from": 1, "to": 1}), json!({"id": id.0})] {
        assert!(s.execute("design.gather", &p).is_err(), "{p}");
    }
}

#[test]
fn a_flat_collar_from_the_bodice_block_is_as_long_as_its_neckline() {
    let mut s = Session::new();
    s.execute("file.new", &json!({"width": 1400, "height": 1000})).unwrap();
    s.execute(
        "measure.set",
        &json!({"values": {"bust": 880, "waist": 680, "nape_to_waist": 410, "back_width": 345, "chest": 325, "shoulder": 122, "neck": 370, "armscye_depth": 210}}),
    )
    .unwrap();
    let r = s.execute("block.bodice", &json!({"origin": [0, 0]})).unwrap();
    let (back, front) = (r["ids"][0].as_u64().unwrap(), r["ids"][1].as_u64().unwrap());
    let c = s.execute("design.collar", &json!({"back": {"id": back}, "front": {"id": front}})).unwrap();
    let id = NodeId(c["id"].as_u64().unwrap());
    assert_eq!(s.doc().unwrap().doc.node(id).unwrap().name.as_deref(), Some("Collar"));
    // The neck edge is the back and front necklines together.
    let back_neck = s.execute("design.length", &json!({"id": back, "from": 1, "to": 2})).unwrap()["mm"].as_f64().unwrap();
    let front_neck = s.execute("design.length", &json!({"id": front, "from": 12, "to": 0})).unwrap()["mm"].as_f64().unwrap();
    assert!((c["neck"].as_f64().unwrap() - (back_neck + front_neck)).abs() < 0.01, "{c} {back_neck} {front_neck}");
    // It starts at the back's centre back neck point and lies over the back's neck.
    let collar = points(&s, id);
    assert_eq!(collar[0], points(&s, NodeId(back))[1]);
    // Lowered for ease: a longer neck edge; undo takes the collar away; bad input refused.
    let low = s.execute("design.collar", &json!({"back": {"id": back}, "front": {"id": front}, "lower": 10})).unwrap();
    assert!(low["neck"].as_f64().unwrap() > c["neck"].as_f64().unwrap());
    s.execute("edit.undo", &json!({})).unwrap();
    assert!(s.doc().unwrap().doc.node(NodeId(low["id"].as_u64().unwrap())).is_none());
    // From the selection: the same collar.
    s.execute("select.set", &json!({"ids": [front, back]})).unwrap();
    let sel = s.execute("design.collar", &json!({})).unwrap();
    assert_eq!(points(&s, NodeId(sel["id"].as_u64().unwrap())), collar);
    for p in [
        json!({"back": {"id": back}}),
        json!({"back": {"id": back}, "front": {"id": front}, "width": 0}),
        json!({"back": {"id": back}, "front": {"id": front, "shoulder": 99}}),
    ] {
        assert!(s.execute("design.collar", &p).is_err(), "{p}");
    }
}

#[test]
fn a_patch_pocket_and_its_placement_on_the_skirt() {
    let (mut s, id, before) = skirt_front();
    s.execute("select.set", &json!({"ids": [id.0]})).unwrap();
    let g = NodeId(s.execute("piece.make", &json!({"name": "Skirt Front"})).unwrap()["ids"][0].as_u64().unwrap());
    let at = Point::new(before[2].x + mm_to_pt(40.0), before[2].y - mm_to_pt(400.0));
    let r = s
        .execute("design.pocket", &json!({"width": 140, "depth": 160, "corners": "round", "origin": [1200, 100], "on": g.0, "at": [at.x, at.y]}))
        .unwrap();
    let (pocket, fold, placement) =
        (NodeId(r["id"].as_u64().unwrap()), NodeId(r["fold"].as_u64().unwrap()), NodeId(r["placement"].as_u64().unwrap()));
    // The piece: 14 cm wide, 16 cm and the 2.5 cm hem tall, where it was asked for.
    let b = bounds(&s, pocket);
    assert!((b.width() - mm_to_pt(140.0)).abs() < 1e-6 && (b.height() - mm_to_pt(185.0)).abs() < 1e-6, "{b:?}");
    assert!((b.x0 - 1200.0).abs() < 1e-6 && (b.y0 - 100.0).abs() < 1e-6);
    let doc = &s.doc().unwrap().doc;
    assert_eq!(doc.node(pocket).unwrap().name.as_deref(), Some("Patch Pocket"));
    assert!(doc.node(fold).unwrap().appearance.stroke().is_some_and(|st| st.dash.is_some()));
    // The placement: the finished pocket, in the skirt piece, its top-left where asked.
    assert_eq!(doc.parent_of(placement), Some(g));
    let m = bounds(&s, placement);
    assert!((m.x0 - at.x).abs() < 1e-6 && (m.y0 - at.y).abs() < 1e-6 && (m.height() - mm_to_pt(160.0)).abs() < 1e-6);
    // It is one undo step; bad input is refused.
    s.execute("edit.undo", &json!({})).unwrap();
    assert!(s.doc().unwrap().doc.node(pocket).is_none() && s.doc().unwrap().doc.node(placement).is_none());
    for p in [
        json!({"width": 140}),
        json!({"width": 140, "depth": 160, "corners": "oval"}),
        json!({"width": 140, "depth": 160, "on": g.0}),
        json!({"width": 140, "depth": 160, "corners": "round", "radius": 90}),
    ] {
        assert!(s.execute("design.pocket", &p).is_err(), "{p}");
    }
}

#[test]
fn walking_the_skirt_side_seams_notches_both_alike() {
    let mut s = Session::new();
    s.execute("file.new", &json!({"width": 1400, "height": 1000})).unwrap();
    s.execute("measure.set", &json!({"values": {"waist": 680, "hip": 940, "waist_to_hip": 200, "skirt_length": 600}})).unwrap();
    let r = s.execute("block.skirt", &json!({"origin": [0, 0]})).unwrap();
    let (back, front) = (r["ids"][0].as_u64().unwrap(), r["ids"][1].as_u64().unwrap());
    // Before they are pieces: lengths only. The back's side seam runs from its top (4) down to
    // the hem (6); the front's from its top (4) back round to the hem (2), the other way along
    // its outline.
    let seams = json!({"a": {"id": back, "from": 4, "to": 6}, "b": {"id": front, "from": 4, "to": 2}});
    let w = s.execute("design.walk", &seams).unwrap();
    assert!(w["difference"].as_f64().unwrap().abs() < 0.01 && w["a"].as_f64().unwrap() > 600.0, "{w}");
    let mut with = seams.clone();
    with["every"] = json!(100);
    assert!(s.execute("design.walk", &with).is_err());
    // As pieces: a notch every 10 cm on both, in one undo step.
    s.execute("select.set", &json!({"ids": [back, front]})).unwrap();
    let pieces = s.execute("piece.make", &json!({})).unwrap();
    let (gb, gf) = (pieces["ids"][0].as_u64().unwrap(), pieces["ids"][1].as_u64().unwrap());
    let w = s.execute("design.walk", &with).unwrap();
    let count = w["notches"].as_u64().unwrap();
    assert_eq!(count, (w["a"].as_f64().unwrap() / 100.0).floor() as u64);
    // A piece without notches leaves the list out.
    let notches = |g: u64, s: &mut Session| s.execute("piece.info", &json!({"id": g})).unwrap()["notches"].as_array().cloned().unwrap_or_default();
    let (nb, nf) = (notches(gb, &mut s), notches(gf, &mut s));
    assert_eq!((nb.len() as u64, nf.len() as u64), (count, count));
    // The first, 10 cm down the hip curve: the back's edge 4 from its start, the front's edge 3
    // from its end.
    let (b0, f0) = (&nb[0], &nf[nf.len() - 1]);
    assert_eq!((b0["edge"].as_u64(), f0["edge"].as_u64()), (Some(4), Some(3)), "{nb:?} {nf:?}");
    assert!((b0["at"].as_f64().unwrap() - (1.0 - f0["at"].as_f64().unwrap())).abs() < 1e-6);
    s.execute("edit.undo", &json!({})).unwrap();
    assert!(notches(gb, &mut s).is_empty());
    let mut far = seams.clone();
    far["notches"] = json!([5000]);
    assert!(s.execute("design.walk", &far).is_err());
}

#[test]
fn smoothing_the_skirt_waist_across_the_side_seam() {
    let mut s = Session::new();
    s.execute("file.new", &json!({"width": 1400, "height": 1000})).unwrap();
    s.execute("measure.set", &json!({"values": {"waist": 680, "hip": 940, "waist_to_hip": 200, "skirt_length": 600}})).unwrap();
    let r = s.execute("block.skirt", &json!({"origin": [0, 0]})).unwrap();
    let (back, front) = (NodeId(r["ids"][0].as_u64().unwrap()), NodeId(r["ids"][1].as_u64().unwrap()));
    let (pb, pf) = (points(&s, back), points(&s, front));
    // The waist meets the side seam at 4 on both; the seam runs on to 5 on the back, 3 on the
    // front.
    let seams = json!({"a": {"id": back.0, "at": 4, "seam": 5}, "b": {"id": front.0, "at": 4, "seam": 3}});
    let first = s.execute("design.smooth", &seams).unwrap()["corner"].as_f64().unwrap();
    assert!((0.0..90.0).contains(&first), "{first}");
    // The points stay put; once smoothed there is no corner left.
    assert_eq!((points(&s, back), points(&s, front)), (pb.clone(), pf.clone()));
    let again = s.execute("design.smooth", &seams).unwrap()["corner"].as_f64().unwrap();
    assert!(again.abs() < 1e-6, "{again}");
    // One undo step each.
    s.execute("edit.undo", &json!({})).unwrap();
    s.execute("edit.undo", &json!({})).unwrap();
    let again = s.execute("design.smooth", &seams).unwrap()["corner"].as_f64().unwrap();
    assert!((again - first).abs() < 1e-9);
    // The seam's other end must be a neighbour; a and b are two pieces.
    for bad in [
        json!({"a": {"id": back.0, "at": 4, "seam": 7}, "b": {"id": front.0, "at": 4, "seam": 3}}),
        json!({"a": {"id": back.0, "at": 4, "seam": 5}, "b": {"id": back.0, "at": 4, "seam": 3}}),
        json!({"a": {"id": back.0, "at": 4}, "b": {"id": front.0, "at": 4, "seam": 3}}),
    ] {
        assert!(s.execute("design.smooth", &bad).is_err(), "{bad}");
    }
}

#[test]
fn a_box_pleat_in_the_skirt_front() {
    let (mut s, id, before) = skirt_front();
    let top = Point::new((before[7].x + before[0].x) / 2.0, (before[7].y + before[0].y) / 2.0);
    let hem = Point::new(top.x, before[1].y);
    let r = s.execute("design.pleat", &json!({"id": id.0, "from": [hem.x, hem.y], "to": [top.x, top.y], "depth": 25, "kind": "box"})).unwrap();
    let after = points(&s, id);
    // Four times the depth: 10 cm more at the hem.
    assert!(((after[after.len() - 1] - after[0]).hypot() - mm_to_pt(100.0)).abs() < 1e-9);
    let doc = &s.doc().unwrap().doc;
    let (fold, placement) = (NodeId(r["fold"].as_u64().unwrap()), NodeId(r["placement"].as_u64().unwrap()));
    let subpaths = |id: NodeId| match &doc.node(id).unwrap().kind {
        NodeKind::Path { path, .. } => path.subpaths.clone(),
        _ => panic!("not a path"),
    };
    // Two fold lines and two arrows; two placement lines, 25 mm outside the spread at the hem.
    assert_eq!((subpaths(fold).len(), subpaths(placement).len()), (6, 2));
    let off: Vec<f64> = subpaths(placement).iter().map(|sp| pf_core::pt_to_mm((sp.anchors[0].p - hem).hypot())).collect();
    assert!((off[0] - 25.0).abs() < 1e-6 && (off[1] - 125.0).abs() < 1e-6, "{off:?}");
    assert!(s.execute("design.pleat", &json!({"id": id.0, "from": [hem.x, hem.y], "to": [top.x, top.y], "depth": 25, "kind": "accordion"})).is_err());
}

#[test]
fn a_stand_collar_for_the_bodice_block() {
    let mut s = Session::new();
    s.execute("file.new", &json!({"width": 1400, "height": 1000})).unwrap();
    s.execute(
        "measure.set",
        &json!({"values": {"bust": 880, "waist": 680, "nape_to_waist": 410, "back_width": 345, "chest": 325, "shoulder": 122, "neck": 370, "armscye_depth": 210}}),
    )
    .unwrap();
    let r = s.execute("block.bodice", &json!({"origin": [0, 0]})).unwrap();
    let (back, front) = (r["ids"][0].as_u64().unwrap(), r["ids"][1].as_u64().unwrap());
    let back_neck = s.execute("design.length", &json!({"id": back, "from": 1, "to": 2})).unwrap()["mm"].as_f64().unwrap();
    let front_neck = s.execute("design.length", &json!({"id": front, "from": 12, "to": 0})).unwrap()["mm"].as_f64().unwrap();
    // From the selection, the bodice block's own neck points.
    s.execute("select.set", &json!({"ids": [back, front]})).unwrap();
    let r = s.execute("design.standCollar", &json!({"extension": 15})).unwrap();
    let neck = r["neck"].as_f64().unwrap();
    assert!((neck - (back_neck + front_neck)).abs() < 0.01, "{r} {back_neck} {front_neck}");
    assert!((r["shoulder"].as_f64().unwrap() - back_neck).abs() < 0.01);
    let id = NodeId(r["id"].as_u64().unwrap());
    assert_eq!(s.doc().unwrap().doc.node(id).unwrap().name.as_deref(), Some("Stand Collar"));
    // Its neck edge, centre back to the front end, is the neckline and the extension long.
    let edge = length(&mut s, id, 0, 3);
    assert!((edge - (neck + 15.0)).abs() < 0.05, "{edge}");
    // The stand 35 mm high at centre back.
    let p = points(&s, id);
    assert!((pf_core::pt_to_mm((p[p.len() - 1] - p[0]).hypot()) - 35.0).abs() < 1e-6);
    assert!(s.execute("design.standCollar", &json!({"height": -5})).is_err());
    s.execute("edit.undo", &json!({})).unwrap();
    assert!(s.doc().unwrap().doc.node(id).is_none());
}

#[test]
fn an_in_seam_pocket_in_the_skirt_side_seam() {
    let mut s = Session::new();
    s.execute("file.new", &json!({"width": 1400, "height": 1000})).unwrap();
    s.execute("measure.set", &json!({"values": {"waist": 680, "hip": 940, "waist_to_hip": 200, "skirt_length": 600}})).unwrap();
    let r = s.execute("block.skirt", &json!({"origin": [0, 0]})).unwrap();
    let (back, front) = (r["ids"][0].as_u64().unwrap(), r["ids"][1].as_u64().unwrap());
    // On the paths: the bag, no notches.
    let seams = json!({"seam": {"id": front, "from": 4, "to": 2}, "match": {"id": back, "from": 4, "to": 6}});
    let r = s.execute("design.inSeamPocket", &seams).unwrap();
    assert_eq!(r["notches"], 0);
    let bag = NodeId(r["id"].as_u64().unwrap());
    assert_eq!(s.doc().unwrap().doc.node(bag).unwrap().name.as_deref(), Some("Pocket Bag"));
    // Its seam edge 140 + 2 × 15 mm along the side seam, the bag inside the front.
    assert!((length(&mut s, bag, 0, 16) - 170.0).abs() < 0.1, "{}", length(&mut s, bag, 0, 16));
    let (fb, bb) = (bounds(&s, NodeId(front)), bounds(&s, bag));
    assert!(bb.x0 >= fb.x0 - 1e-6 && bb.x1 <= fb.x1 + 1e-6 && bb.y0 >= fb.y0 && bb.y1 <= fb.y1, "{fb:?} {bb:?}");
    s.execute("edit.undo", &json!({})).unwrap();
    // On pieces: notches at the opening's ends on both, in the same undo step as the bag.
    s.execute("select.set", &json!({"ids": [back, front]})).unwrap();
    let pieces = s.execute("piece.make", &json!({})).unwrap();
    let (gb, gf) = (pieces["ids"][0].as_u64().unwrap(), pieces["ids"][1].as_u64().unwrap());
    let r = s.execute("design.inSeamPocket", &seams).unwrap();
    assert_eq!(r["notches"], 4);
    let notches = |g: u64, s: &mut Session| s.execute("piece.info", &json!({"id": g})).unwrap()["notches"].as_array().cloned().unwrap_or_default();
    assert_eq!((notches(gb, &mut s).len(), notches(gf, &mut s).len()), (2, 2));
    s.execute("edit.undo", &json!({})).unwrap();
    assert!(notches(gf, &mut s).is_empty());
    for bad in [
        json!({"seam": {"id": front, "from": 4, "to": 2}, "top": 900}),
        json!({"seam": {"id": front, "from": 4, "to": 2}, "width": 5}),
        json!({"top": 90}),
    ] {
        assert!(s.execute("design.inSeamPocket", &bad).is_err(), "{bad}");
    }
}

#[test]
fn a_welt_pocket_marked_on_the_skirt_front() {
    let (mut s, id, before) = skirt_front();
    let at = Point::new((before[0].x + before[7].x) / 2.0, before[0].y + mm_to_pt(80.0));
    let r = s.execute("design.weltPocket", &json!({"welts": "double", "on": id.0, "at": [at.x, at.y]})).unwrap();
    let ids: Vec<NodeId> = r["ids"].as_array().unwrap().iter().map(|v| NodeId(v.as_u64().unwrap())).collect();
    let names: Vec<Option<String>> = ids.iter().map(|i| s.doc().unwrap().doc.node(*i).unwrap().name.clone()).collect();
    assert_eq!(names, [Some("Welt (cut 2)".into()), Some("Pocket bag, lower".into()), Some("Pocket bag, upper".into())]);
    // The welt strip 120 + 2 × 15 long, 4 × 7 high; the opening 120 by 14 at `at`.
    let mm = |r: Rect| (pf_core::pt_to_mm(r.width()), pf_core::pt_to_mm(r.height()));
    let (w, h) = mm(bounds(&s, ids[0]));
    assert!((w - 150.0).abs() < 1e-6 && (h - 28.0).abs() < 1e-6, "{w} {h}");
    let opening = NodeId(r["opening"].as_u64().unwrap());
    let ob = bounds(&s, opening);
    assert!(((ob.x0 - at.x).abs() < 1e-6) && (ob.y0 - at.y).abs() < 1e-6);
    let (w, h) = mm(ob);
    assert!((w - 120.0).abs() < 1e-6 && (h - 14.0).abs() < 1e-6, "{w} {h}");
    // One undo step takes it all away.
    s.execute("edit.undo", &json!({})).unwrap();
    assert!(s.doc().unwrap().doc.node(ids[0]).is_none() && s.doc().unwrap().doc.node(opening).is_none());
    assert!(s.execute("design.weltPocket", &json!({"welts": "triple"})).is_err());
    assert!(s.execute("design.weltPocket", &json!({"on": id.0})).is_err());
}
