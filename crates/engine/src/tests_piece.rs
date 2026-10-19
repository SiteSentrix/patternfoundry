//! Pattern pieces (PF-1, `piece.*`): a closed path becomes a piece whose cutting line, notches,
//! grainline and label are drawn from its seam line and follow it.

use pf_core::{Mark, PatternTag, mm_to_pt};
use serde_json::{Value, json};
use vectorcraft_doc::{Node, NodeId, NodeKind};
use vectorcraft_geom::Rect;

use super::*;

/// 100 mm, in points.
fn cm10() -> f64 {
    mm_to_pt(100.0)
}

/// A session with a 10 cm square at (0, 0), selected.
fn square() -> (Session, NodeId) {
    let mut s = Session::new();
    s.execute("file.new", &json!({"width": 800, "height": 800})).unwrap();
    let r = s.execute("shape.rectangle", &json!({"x": 0, "y": 0, "width": cm10(), "height": cm10()})).unwrap();
    (s, NodeId(r["id"].as_u64().unwrap()))
}

fn make(s: &mut Session, p: Value) -> NodeId {
    let r = s.execute("piece.make", &p).unwrap();
    NodeId(r["ids"][0].as_u64().unwrap())
}

fn node(s: &Session, id: NodeId) -> Node {
    s.doc().unwrap().doc.node(id).unwrap().clone()
}

/// The child of the piece `id` that draws `mark`.
fn part(s: &Session, id: NodeId, mark: Mark) -> Option<Node> {
    let g = node(s, id);
    g.children()?.iter().find(|c| c.extras().pattern.as_deref().and_then(PatternTag::mark) == Some(mark)).map(|c| (**c).clone())
}

fn bounds(n: &Node) -> Rect {
    match &n.kind {
        NodeKind::Path { path, .. } => path.bounds().unwrap(),
        _ => panic!("not a path"),
    }
}

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-6
}

#[test]
fn a_square_becomes_a_piece_with_its_cutting_line_one_allowance_out() {
    let (mut s, rect) = square();
    let g = make(&mut s, json!({"allowance": 10, "name": "Front"}));
    let group = node(&s, g);
    assert!(group.extras().pattern.as_deref().and_then(PatternTag::piece).is_some());
    assert_eq!(group.name.as_deref(), Some("Front"));
    // The seam line is the path that was drawn, same id, editable.
    let seam = part(&s, g, Mark::Seam).unwrap();
    assert_eq!(seam.id, rect);
    assert!(!seam.locked);
    let cut = part(&s, g, Mark::Cut).unwrap();
    assert!(cut.locked, "generated parts are locked");
    let (b, w) = (bounds(&cut), mm_to_pt(10.0));
    assert!(near(b.x0, -w) && near(b.y0, -w) && near(b.x1, cm10() + w) && near(b.y1, cm10() + w), "{b:?}");
    assert!(part(&s, g, Mark::Grainline).is_some() && part(&s, g, Mark::Label).is_some());
    // Paint order: cutting line under the seam line, the label on top.
    let order: Vec<Option<Mark>> = group.children().unwrap().iter().map(|c| c.extras().pattern.as_deref().and_then(PatternTag::mark)).collect();
    assert_eq!(order, [Some(Mark::Cut), Some(Mark::Seam), Some(Mark::Grainline), Some(Mark::Label)]);
    let info = s.execute("piece.info", &json!({})).unwrap();
    assert_eq!((info["name"].as_str(), info["cut"].as_u64(), info["edgeCount"].as_u64()), (Some("Front"), Some(1), Some(4)));
    assert_eq!(info["seamLength"], json!(400.0));
    assert_eq!(info["area"], json!(100.0));
    assert!(info["problem"].is_null());
    let list = s.execute("piece.list", &json!({})).unwrap();
    assert_eq!(list["pieces"][0]["id"], json!(g.0));
}

#[test]
fn the_cutting_line_follows_when_the_seam_line_moves_and_undo_takes_it_back() {
    let (mut s, rect) = square();
    let g = make(&mut s, json!({}));
    // Edit the seam line alone (as the Direct Selection tool does).
    s.execute("select.set", &json!({"ids": [rect.0]})).unwrap();
    s.execute("object.move", &json!({"dx": 50, "dy": 0})).unwrap();
    let b = bounds(&part(&s, g, Mark::Cut).unwrap());
    assert!(near(b.x0, 50.0 - mm_to_pt(10.0)), "{b:?}");
    s.execute("edit.undo", &json!({})).unwrap();
    let b = bounds(&part(&s, g, Mark::Cut).unwrap());
    assert!(near(b.x0, -mm_to_pt(10.0)), "{b:?}");
    // Undo the piece itself: the plain square is back.
    s.execute("edit.undo", &json!({})).unwrap();
    assert!(s.doc().unwrap().doc.node(g).is_none());
    assert!(node(&s, rect).extras().pattern.is_none());
}

#[test]
fn moving_the_piece_moves_its_grainline() {
    let (mut s, _) = square();
    let g = make(&mut s, json!({}));
    s.execute("piece.set", &json!({"grainline": {"from": [20, 20], "to": [20, 200]}})).unwrap();
    s.execute("select.set", &json!({"ids": [g.0]})).unwrap();
    s.execute("object.move", &json!({"dx": 10, "dy": 5})).unwrap();
    let info = s.execute("piece.info", &json!({})).unwrap();
    assert_eq!(info["grainline"], json!({"from": {"x": 30.0, "y": 25.0}, "to": {"x": 30.0, "y": 205.0}}));
    let b = bounds(&part(&s, g, Mark::Grainline).unwrap());
    assert!(near(b.y0, 25.0) && near(b.y1, 205.0), "{b:?}");
}

#[test]
fn each_edge_and_corner_takes_its_own_settings() {
    let (mut s, _) = square();
    let g = make(&mut s, json!({"allowance": "5/8 in"}));
    let info = s.execute("piece.info", &json!({})).unwrap();
    assert!(near(info["allowance"].as_f64().unwrap(), 15.875));
    // The rectangle's edges run top, right, bottom, left: the left edge is on the fold (no
    // allowance) and the hem (bottom) is 4 cm.
    let info = s
        .execute(
            "piece.set",
            &json!({"allowance": 10, "edges": {"2": 40, "3": 0}, "corners": {"1": "square"}, "cut": 2, "mirror": true, "fabric": "Linen"}),
        )
        .unwrap();
    assert_eq!((info["cut"].as_u64(), info["mirror"].as_bool(), info["fabric"].as_str()), (Some(2), Some(true), Some("Linen")));
    let b = bounds(&part(&s, g, Mark::Cut).unwrap());
    assert!(near(b.x0, 0.0) && near(b.y0, -mm_to_pt(10.0)) && near(b.y1, cm10() + mm_to_pt(40.0)), "{b:?}");
    // Back to the piece's own allowance.
    s.execute("piece.set", &json!({"edges": {"2": null, "3": null}})).unwrap();
    let b = bounds(&part(&s, g, Mark::Cut).unwrap());
    assert!(near(b.x0, -mm_to_pt(10.0)), "{b:?}");
    let label = part(&s, g, Mark::Label).unwrap();
    let NodeKind::Text(t) = &label.kind else { panic!("label is type") };
    let text: String = t.runs.iter().map(|r| r.text.as_str()).collect();
    assert!(text.contains("Cut 2 (mirrored pairs) · Linen"), "{text}");
}

#[test]
fn notches_are_snipped_into_the_allowance() {
    let (mut s, _) = square();
    let g = make(&mut s, json!({}));
    s.execute("piece.addNotch", &json!({"edge": 1, "at": 0.5})).unwrap();
    let info = s.execute("piece.addNotch", &json!({"edge": 0, "kind": "double"})).unwrap();
    assert_eq!(info["notches"].as_array().unwrap().len(), 2);
    let notches = part(&s, g, Mark::Notches).unwrap();
    let NodeKind::Path { path, .. } = &notches.kind else { panic!() };
    assert_eq!(path.subpaths.len(), 3, "a slit and a double notch");
    // The right edge's slit: from the cutting line 6 mm in, half way down.
    let b = bounds(&notches);
    assert!(near(b.x1, cm10() + mm_to_pt(10.0)), "{b:?}");
    assert!(s.execute("piece.addNotch", &json!({"edge": 9})).is_err(), "the square has edges 0 to 3");
    s.execute("piece.removeNotch", &json!({"index": 0})).unwrap();
    s.execute("piece.removeNotch", &json!({"index": 0})).unwrap();
    assert!(part(&s, g, Mark::Notches).is_none());
    assert!(s.execute("piece.removeNotch", &json!({"index": 0})).is_err());
}

#[test]
fn a_piece_survives_saving_and_releasing_gives_back_the_path() {
    let (mut s, rect) = square();
    make(&mut s, json!({"name": "Back"}));
    s.execute("piece.addNotch", &json!({"edge": 2})).unwrap();
    let doc = s.doc().unwrap().doc.clone();
    let text = serde_json::to_string(&*doc).unwrap();
    let back: vectorcraft_doc::Document = serde_json::from_str(&text).unwrap();
    assert!(back.pieces, "the document says it has pieces");
    assert_eq!(serde_json::to_value(&back).unwrap(), serde_json::to_value(&*doc).unwrap());
    let r = s.execute("piece.release", &json!({})).unwrap();
    assert_eq!(r["ids"], json!([rect.0]));
    let n = node(&s, rect);
    assert!(n.extras().pattern.is_none() && matches!(n.kind, NodeKind::Path { .. }));
    assert_eq!(s.execute("piece.list", &json!({})).unwrap()["pieces"], json!([]));
}

#[test]
fn bad_requests_are_refused_without_changing_anything() {
    let (mut s, _) = square();
    assert!(s.execute("piece.make", &json!({"allowance": -2})).is_err());
    assert!(s.execute("piece.make", &json!({"allowance": "lots"})).is_err());
    assert!(s.execute("piece.info", &json!({})).is_err(), "not a piece yet");
    // An open path is no piece.
    s.execute("file.new", &json!({"width": 100, "height": 100})).unwrap();
    s.execute("path.create", &json!({"anchors": [{"x": 0, "y": 0}, {"x": 50, "y": 50}, {"x": 90, "y": 0}], "closed": false})).unwrap();
    let e = s.execute("piece.make", &json!({})).unwrap_err().to_string();
    assert!(e.contains("closed paths"), "{e}");
    let (mut s, _) = square();
    make(&mut s, json!({}));
    let before = serde_json::to_value(&*s.doc().unwrap().doc).unwrap();
    for p in [
        json!({"cut": 0}),
        json!({"cut": "two"}),
        json!({"edges": {"x": 3}}),
        json!({"corners": {"1": "round"}}),
        json!({"grainline": {"from": [1, 1], "to": [1, 1]}}),
        json!({"allowance": 1e9}),
        json!({"name": "n".repeat(500)}),
    ] {
        assert!(s.execute("piece.set", &p).is_err(), "{p}");
    }
    assert!(s.execute("piece.addNotch", &json!({"edge": 0, "at": 2})).is_err());
    assert_eq!(serde_json::to_value(&*s.doc().unwrap().doc).unwrap(), before);
}

#[test]
fn a_piece_exports_with_its_cutting_line_notches_grainline_and_label() {
    let (mut s, _) = square();
    make(&mut s, json!({"name": "Yoke"}));
    s.execute("piece.addNotch", &json!({"edge": 1})).unwrap();
    let r = s.execute("document.export", &json!({"format": "svg"})).unwrap();
    let svg = String::from_utf8(vectorcraft_format::base64_decode(r["dataBase64"].as_str().unwrap()).unwrap()).unwrap();
    // The cutting line, the dashed seam line, the grainline and the notch are paths; the label is type.
    assert!(svg.matches("<path").count() + svg.matches("<polygon").count() + svg.matches("<rect").count() >= 4, "{svg}");
    assert!(svg.contains("stroke-dasharray"), "the seam line is dashed: {svg}");
    assert!(svg.contains("Yoke"), "{svg}");
}
