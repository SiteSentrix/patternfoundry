//! SVG for cutting (PF-7.2, `document.export {format: svgcut}`): pattern pieces' cutting lines,
//! notches and marks in millimetres, true to scale in any SVG reader.

use pf_core::{mm_to_pt, pt_to_mm};
use serde_json::{Value, json};
use vectorcraft_doc::NodeKind;

use super::*;

/// A 10 cm square made a piece with a 10 mm allowance and a notch in its top edge.
fn square_piece() -> Session {
    let mut s = Session::new();
    s.execute("file.new", &json!({"width": 800, "height": 800})).unwrap();
    let side = mm_to_pt(100.0);
    s.execute("shape.rectangle", &json!({"x": 40, "y": 40, "width": side, "height": side})).unwrap();
    s.execute("piece.make", &json!({"name": "Front & Back", "allowance": 10})).unwrap();
    s.execute("piece.addNotch", &json!({"edge": 0, "at": 0.5})).unwrap();
    s
}

fn export(s: &mut Session, p: Value) -> String {
    let r = s.execute("document.export", &p).unwrap();
    String::from_utf8(vectorcraft_format::base64_decode(r["dataBase64"].as_str().unwrap()).unwrap()).unwrap()
}

#[test]
fn the_10_cm_square_cuts_120_mm_and_the_page_is_in_millimetres() {
    let mut s = square_piece();
    let svg = export(&mut s, json!({"format": "svgcut"}));
    // The page: the cutting line and 10 mm round it.
    assert!(svg.contains("width=\"140mm\" height=\"140mm\" viewBox=\"0 0 140 140\""), "{svg}");
    // Read back by an SVG reader: the page and the cutting line at their sizes.
    let doc = vectorcraft_svg::import(&svg).unwrap();
    let page = doc.artboards[0].rect;
    assert!((pt_to_mm(page.width()) - 140.0).abs() < 0.01, "{}", pt_to_mm(page.width()));
    let mut cut = None;
    doc.walk(|n| {
        if let NodeKind::Path { path, .. } = &n.kind
            && path.subpaths.first().is_some_and(|sp| sp.closed)
            && cut.is_none()
        {
            cut = n.geometric_bounds();
        }
    });
    let cut = cut.unwrap();
    assert!((pt_to_mm(cut.width()) - 120.0).abs() < 0.01 && (pt_to_mm(cut.height()) - 120.0).abs() < 0.01, "{cut:?}");
    // One notch, 6 mm into the allowance from the top of the cutting line.
    assert!(svg.contains("<g id=\"notches\"") && svg.contains("<path d=\"M70 10 L70 16\"/>"), "{svg}");
    // The marks: the dashed seam line 10 mm in, the grainline, the name (escaped).
    assert!(svg.contains("stroke-dasharray") && svg.contains("M20 20 L120 20 L120 120 L20 120 Z"), "{svg}");
    assert!(svg.contains(">Front &amp; Back</tspan>") && svg.contains(">Cut 1</tspan>"), "{svg}");
}

#[test]
fn without_marks_only_the_cuts_are_written() {
    let mut s = square_piece();
    let svg = export(&mut s, json!({"format": "svgcut", "marks": false}));
    assert!(svg.contains("<g id=\"cut\"") && svg.contains("<g id=\"notches\"") && !svg.contains("<g id=\"draw\""), "{svg}");
    let mut t = Session::new();
    t.execute("file.new", &json!({"width": 800, "height": 800})).unwrap();
    t.execute("shape.rectangle", &json!({"x": 0, "y": 0, "width": 100, "height": 100})).unwrap();
    assert!(t.execute("document.export", &json!({"format": "svgcut"})).unwrap_err().to_string().contains("pattern pieces"));
}
