//! Grading by redrafting (PF-6.1, `grade.nest`): the pattern drafted again in each size of the
//! size chart, nested on a layer per size.

use serde_json::{Value, json};
use vectorcraft_doc::{Node, NodeKind};

use super::*;

const CHART: &str = "size, waist, hip, waist_to_hip, skirt_length\n10, 66, 92, 20, 60\n12, 70, 96, 20.5, 60\n14, 74, 100, 21, 60\n";

/// The skirt block, drafted in size 12, made pattern pieces.
fn skirt() -> Session {
    let mut s = Session::new();
    s.execute("file.new", &json!({"width": 1400, "height": 1000})).unwrap();
    s.execute("measure.importChart", &json!({"data": CHART, "name": "Chart"})).unwrap();
    s.execute("measure.useSize", &json!({"size": "12"})).unwrap();
    s.execute("block.skirt", &json!({"origin": [40, 40]})).unwrap();
    s.execute("select.all", &json!({})).unwrap();
    s.execute("piece.make", &json!({"allowance": 15})).unwrap();
    s
}

fn nest_layer(s: &Session) -> Node {
    let doc = &s.doc().unwrap().doc;
    let n: Vec<&std::sync::Arc<Node>> = doc.layers.iter().filter(|l| l.name.as_deref() == Some("Graded Nest")).collect();
    assert_eq!(n.len(), 1, "one nest");
    (**n[0]).clone()
}

fn widths(v: &Value, piece: usize) -> Vec<f64> {
    v["sizes"].as_array().unwrap().iter().map(|s| s["pieces"][piece]["widthMm"].as_f64().unwrap()).collect()
}

/// The cutting lines of the document's own pieces.
fn own_cuts(s: &Session) -> Vec<vectorcraft_geom::PathData> {
    let mut out = Vec::new();
    s.doc().unwrap().doc.walk(|n| {
        if matches!(n.extras().pattern.as_deref(), Some(pf_core::PatternTag::Mark(pf_core::Mark::Cut)))
            && let NodeKind::Path { path, .. } = &n.kind
        {
            out.push(path.clone());
        }
    });
    out
}

#[test]
fn every_size_is_drafted_again_on_a_layer_of_its_own() {
    let mut s = skirt();
    let measured = s.execute("measure.info", &json!({})).unwrap();
    let v = s.execute("grade.nest", &json!({})).unwrap();
    assert_eq!(v["warnings"], json!([]));
    let sizes: Vec<&str> = v["sizes"].as_array().unwrap().iter().map(|s| s["size"].as_str().unwrap()).collect();
    assert_eq!(sizes, ["10", "12", "14"]);
    // Each piece grows with the size: a quarter of the 40 mm hip step, give or take the ease.
    for piece in 0..2 {
        let w = widths(&v, piece);
        assert!(w[0] < w[1] && w[1] < w[2], "{w:?}");
        for step in [w[1] - w[0], w[2] - w[1]] {
            assert!((step - 10.0).abs() < 1.5, "{w:?}");
        }
    }
    // A locked layer with a sublayer per size, each holding both pieces' cutting lines.
    let nest = nest_layer(&s);
    assert!(nest.locked);
    let subs = nest.children().unwrap();
    let names: Vec<&str> = subs.iter().map(|l| l.name.as_deref().unwrap()).collect();
    assert_eq!(names, ["Size 10", "Size 12", "Size 14"]);
    let paths = |l: &Node| -> Vec<(vectorcraft_geom::PathData, f64)> {
        l.children()
            .unwrap()
            .iter()
            .filter_map(|c| match &c.kind {
                NodeKind::Path { path, .. } => Some((path.clone(), c.appearance.stroke().unwrap().width)),
                _ => None,
            })
            .collect()
    };
    assert!(subs.iter().all(|l| paths(l).len() == 2));
    // Size 12, the one the document is drafted in, is the document's own cutting lines, heavier.
    let own = own_cuts(&s);
    let twelve = paths(&subs[1]);
    assert_eq!(twelve.iter().map(|(p, _)| p.clone()).collect::<Vec<_>>(), own);
    assert!(twelve.iter().all(|(_, w)| *w == 1.0) && paths(&subs[0]).iter().all(|(_, w)| *w == 0.5));
    // The document's measurements and pieces are as they were.
    assert_eq!(s.execute("measure.info", &json!({})).unwrap(), measured);
    assert_eq!(own_cuts(&s), own);
    // Again: the nest is replaced, not added; then undone and cleared.
    let v = s.execute("grade.nest", &json!({"sizes": ["14", "10"], "line": "seam"})).unwrap();
    assert_eq!(v["sizes"].as_array().unwrap().len(), 2);
    assert_eq!(nest_layer(&s).children().unwrap().len(), 2);
    s.execute("edit.undo", &json!({})).unwrap();
    assert_eq!(nest_layer(&s).children().unwrap().len(), 3);
    assert_eq!(s.execute("grade.clear", &json!({})).unwrap(), json!({"removed": true}));
    assert!(s.doc().unwrap().doc.layers.iter().all(|l| l.name.as_deref() != Some("Graded Nest")));
}

#[test]
fn a_pdf_with_layers_has_a_layer_per_size() {
    let mut s = skirt();
    s.execute("grade.nest", &json!({})).unwrap();
    let v =
        s.execute("document.exportPdf", &json!({"createLayers": true, "preserveEditing": false, "compression": {"compressText": false}})).unwrap();
    let pdf = vectorcraft_format::base64_decode(v["dataBase64"].as_str().unwrap()).unwrap();
    let text = String::from_utf8_lossy(&pdf);
    for name in ["Graded Nest", "Size 10", "Size 12", "Size 14"] {
        assert!(text.contains(&format!("/Name({name})")), "no PDF layer {name}");
    }
}

#[test]
fn sizes_the_chart_cannot_draft_are_reported_and_bad_requests_refused() {
    let mut s = skirt();
    assert!(s.execute("grade.nest", &json!({"sizes": ["16"]})).unwrap_err().to_string().contains("no size `16`"));
    assert!(s.execute("grade.nest", &json!({"line": "fold"})).unwrap_err().to_string().contains("line"));
    // A chart without the skirt's measurements: every size drafts as the document is.
    s.execute("measure.importChart", &json!({"data": "size, bust\nS, 84\nM, 88\n"})).unwrap();
    let v = s.execute("grade.nest", &json!({})).unwrap();
    assert_eq!(v["warnings"].as_array().unwrap().len(), 2, "{v}");
    let mut t = Session::new();
    t.execute("file.new", &json!({"width": 800, "height": 800})).unwrap();
    assert!(t.execute("grade.nest", &json!({})).unwrap_err().to_string().contains("no sizes"));
    t.execute("measure.importChart", &json!({"data": CHART})).unwrap();
    assert!(t.execute("grade.nest", &json!({})).unwrap_err().to_string().contains("pattern pieces"));
    assert_eq!(t.execute("grade.clear", &json!({})).unwrap(), json!({"removed": false}));
}

#[test]
fn each_piece_lines_up_on_the_point_asked() {
    let mut s = skirt();
    let v = s.execute("grade.nest", &json!({"align": {"front skirt block": "topRight", "*": "topLeft"}})).unwrap();
    let names: Vec<&str> = v["sizes"][0]["pieces"].as_array().unwrap().iter().map(|p| p["name"].as_str().unwrap()).collect();
    assert_eq!(names, ["Back Skirt Block", "Front Skirt Block"]);
    let own: Vec<vectorcraft_geom::Rect> = own_cuts(&s).iter().map(|p| p.bounds().unwrap()).collect();
    let nest = nest_layer(&s);
    for size in nest.children().unwrap() {
        let b: Vec<vectorcraft_geom::Rect> = size.children().unwrap().iter().map(|c| c.path_data().unwrap().bounds().unwrap()).collect();
        // The back on its top left corner (centre back at the waist), the front on its top right.
        assert!((b[0].x0 - own[0].x0).abs() < 1e-6 && (b[0].y0 - own[0].y0).abs() < 1e-6, "{b:?} {own:?}");
        assert!((b[1].x1 - own[1].x1).abs() < 1e-6 && (b[1].y0 - own[1].y0).abs() < 1e-6, "{b:?} {own:?}");
    }
    // As drafted, the front moves out with the back's width.
    s.execute("grade.nest", &json!({})).unwrap();
    let nest = nest_layer(&s);
    let x1: Vec<f64> = nest.children().unwrap().iter().map(|l| l.children().unwrap()[1].path_data().unwrap().bounds().unwrap().x1).collect();
    assert!(x1[0] < x1[1] && x1[1] < x1[2], "{x1:?}");
    assert!(s.execute("grade.nest", &json!({"align": "middle"})).unwrap_err().to_string().contains("align"));
}

/// A 10 cm square piece graded S, M (base), L.
fn square() -> (Session, u64) {
    let mut s = Session::new();
    s.execute("file.new", &json!({"width": 800, "height": 800})).unwrap();
    let side = pf_core::mm_to_pt(100.0);
    s.execute("shape.rectangle", &json!({"x": 40, "y": 40, "width": side, "height": side})).unwrap();
    let id = s.execute("piece.make", &json!({"name": "Square", "allowance": 10})).unwrap()["ids"][0].as_u64().unwrap();
    s.execute("grade.setSizes", &json!({"sizes": ["S", "M", "L"], "base": "m"})).unwrap();
    (s, id)
}

/// The seam line's points of piece `id`, in mm.
fn seam_points(s: &Session, id: u64) -> Vec<[f64; 2]> {
    let doc = &s.doc().unwrap().doc;
    let n = doc.node(vectorcraft_doc::NodeId(id)).unwrap();
    let seam =
        n.children().unwrap().iter().find(|c| matches!(c.extras().pattern.as_deref(), Some(pf_core::PatternTag::Mark(pf_core::Mark::Seam)))).unwrap();
    seam.path_data().unwrap().subpaths[0].anchors.iter().map(|a| [pf_core::pt_to_mm(a.p.x), pf_core::pt_to_mm(a.p.y)]).collect()
}

#[test]
fn grade_rules_move_points_from_size_to_size() {
    let (mut s, id) = square();
    let pts = seam_points(&s, id);
    let right = pts.iter().map(|p| p[0]).fold(f64::MIN, f64::max);
    // The right side's points move 10 mm out a size; the left side's stay.
    for (i, p) in pts.iter().enumerate() {
        let dx = if (p[0] - right).abs() < 1e-6 { 10.0 } else { 0.0 };
        s.execute("grade.setRule", &json!({"id": id, "anchor": i, "step": [dx, 0]})).unwrap();
    }
    let info = s.execute("grade.info", &json!({"id": id})).unwrap();
    assert_eq!(info["base"], json!("M"));
    assert_eq!(info["rules"].as_array().unwrap().len(), pts.len());
    let moved = |size: &str| -> Vec<f64> { info["moves"][size].as_array().unwrap().iter().map(|m| m[0].as_f64().unwrap()).collect() };
    for (i, p) in pts.iter().enumerate() {
        let out = if (p[0] - right).abs() < 1e-6 { 10.0 } else { 0.0 };
        assert_eq!([moved("S")[i], moved("M")[i], moved("L")[i]], [-out, 0.0, out]);
    }
    // The nest: seam lines 90, 100 and 110 mm wide; cutting lines 20 mm more; M is the document's.
    let v = s.execute("grade.nest", &json!({"line": "seam"})).unwrap();
    assert_eq!(widths(&v, 0), [90.0, 100.0, 110.0]);
    assert_eq!(v["warnings"], json!([]));
    let v = s.execute("grade.nest", &json!({})).unwrap();
    assert_eq!(widths(&v, 0), [110.0, 120.0, 130.0]);
    let nest = nest_layer(&s);
    let m = nest.children().unwrap()[1].children().unwrap()[0].path_data().unwrap().clone();
    assert_eq!(vec![m], own_cuts(&s));
    // The document itself is not graded.
    assert_eq!(seam_points(&s, id), pts);
}

#[test]
fn irregular_rules_name_each_size() {
    let (mut s, id) = square();
    let pts = seam_points(&s, id);
    let bottom = pts.iter().map(|p| p[1]).fold(f64::MIN, f64::max);
    for (i, p) in pts.iter().enumerate() {
        if (p[1] - bottom).abs() < 1e-6 {
            s.execute("grade.setRule", &json!({"id": id, "anchor": i, "sizes": {"s": [0, -3], "l": [0, 8]}})).unwrap();
        } else {
            s.execute("grade.setRule", &json!({"id": id, "anchor": i, "step": [0, 0]})).unwrap();
        }
    }
    let v = s.execute("grade.nest", &json!({"line": "seam"})).unwrap();
    let heights: Vec<f64> = v["sizes"].as_array().unwrap().iter().map(|s| s["pieces"][0]["heightMm"].as_f64().unwrap()).collect();
    assert_eq!(heights, [97.0, 100.0, 108.0]);
    // Spelled as the range spells the sizes.
    let info = s.execute("grade.info", &json!({"id": id})).unwrap();
    assert!(info["rules"].as_array().unwrap().iter().any(|r| r["sizes"]["L"] == json!([0.0, 8.0])), "{info}");
    // Cleared, a point goes back to following its neighbours.
    s.execute("grade.setRule", &json!({"id": id, "anchor": 0, "clear": true})).unwrap();
    assert!(s.execute("grade.info", &json!({"id": id})).unwrap()["rules"].as_array().unwrap().iter().all(|r| r["anchor"] != json!(0)));
}

#[test]
fn rules_and_redrafting_add_up() {
    // The skirt drafted in each chart size, and the back moved 5 mm down a size by one rule.
    let mut s = skirt();
    s.execute("grade.setSizes", &json!({"sizes": ["10", "12", "14"], "base": "12"})).unwrap();
    let back = s.execute("grade.nest", &json!({})).unwrap();
    let drafted: Vec<f64> =
        (0..3).map(|i| nest_layer(&s).children().unwrap()[i].children().unwrap()[0].path_data().unwrap().bounds().unwrap().y0).collect();
    assert_eq!(back["warnings"], json!([]));
    let id = {
        let doc = &s.doc().unwrap().doc;
        let mut id = None;
        doc.walk(|n| {
            if id.is_none() && super::cmd::piece::piece_on(n).is_some() {
                id = Some(n.id.0);
            }
        });
        id.unwrap()
    };
    s.execute("grade.setRule", &json!({"id": id, "anchor": 0, "step": [0, 5]})).unwrap();
    s.execute("grade.nest", &json!({})).unwrap();
    let ruled: Vec<f64> =
        (0..3).map(|i| nest_layer(&s).children().unwrap()[i].children().unwrap()[0].path_data().unwrap().bounds().unwrap().y0).collect();
    for (k, (r, d)) in ruled.iter().zip(&drafted).enumerate() {
        assert!((pf_core::pt_to_mm(r - d) - 5.0 * (k as f64 - 1.0)).abs() < 1e-6, "{ruled:?} {drafted:?}");
    }
}

#[test]
fn bad_sizes_and_rules_are_refused() {
    let (mut s, id) = square();
    let e = |s: &mut Session, p: Value| s.execute(p["cmd"].as_str().unwrap(), &p["p"]).unwrap_err().to_string();
    assert!(e(&mut s, json!({"cmd": "grade.setSizes", "p": {"sizes": ["S", "M"], "base": "L"}})).contains("base size"));
    assert!(e(&mut s, json!({"cmd": "grade.setSizes", "p": {"sizes": ["S", "s"], "base": "S"}})).contains("twice"));
    assert!(e(&mut s, json!({"cmd": "grade.setRule", "p": {"id": id, "anchor": 99, "step": [1, 0]}})).contains("points 0 to"));
    assert!(e(&mut s, json!({"cmd": "grade.setRule", "p": {"id": id, "anchor": 0, "step": [1]}})).contains("step"));
    assert!(e(&mut s, json!({"cmd": "grade.setRule", "p": {"id": id, "anchor": 0, "step": [900, 0]}})).contains("-500 to 500"));
    assert!(e(&mut s, json!({"cmd": "grade.setRule", "p": {"id": id, "anchor": 0}})).contains("clear"));
    // Without rules every size draws as the base: a warning for each other size.
    let v = s.execute("grade.nest", &json!({})).unwrap();
    assert_eq!(v["warnings"].as_array().unwrap().len(), 2, "{v}");
    assert_eq!(s.execute("grade.setSizes", &json!({"sizes": null})).unwrap(), json!({"sizes": null}));
    assert!(e(&mut s, json!({"cmd": "grade.nest", "p": {}})).contains("no sizes"));
}

#[test]
fn piece_exports_write_the_size_asked() {
    let (mut s, id) = square();
    let pts = seam_points(&s, id);
    let right = pts.iter().map(|p| p[0]).fold(f64::MIN, f64::max);
    for (i, p) in pts.iter().enumerate() {
        let dx = if (p[0] - right).abs() < 1e-6 { 10.0 } else { 0.0 };
        s.execute("grade.setRule", &json!({"id": id, "anchor": i, "step": [dx, 0]})).unwrap();
    }
    let text = |s: &mut Session, p: Value| {
        let r = s.execute("document.export", &p).unwrap();
        String::from_utf8(vectorcraft_format::base64_decode(r["dataBase64"].as_str().unwrap()).unwrap()).unwrap()
    };
    // SVG for cutting: the page is the cutting line and 10 mm round it.
    for (size, page) in [(None, 140), (Some("S"), 130), (Some("l"), 150)] {
        let svg = text(&mut s, json!({"format": "svgcut", "size": size}));
        assert!(svg.contains(&format!("width=\"{page}mm\"")), "{size:?}: {}", &svg[..300]);
    }
    // DXF-AAMA: the size written is the file's sample size, unless given.
    let dxf = text(&mut s, json!({"format": "aama", "size": "L"}));
    assert!(dxf.contains("Sample Size: L"), "{dxf}");
    let dxf = text(&mut s, json!({"format": "aama", "size": "L", "sampleSize": "Large"}));
    assert!(dxf.contains("Sample Size: Large"));
    // The projector PDF: the 130 mm cutting line of size L, 25 mm in.
    let r = s.execute("document.export", &json!({"format": "projector", "size": "L"})).unwrap();
    let doc = vectorcraft_pdf::import(&vectorcraft_format::base64_decode(r["dataBase64"].as_str().unwrap()).unwrap()).unwrap();
    assert!((pf_core::pt_to_mm(doc.artboards[0].rect.width()) - 180.0).abs() < 0.01);
    // The document is still in its base size; an unknown size is refused.
    assert_eq!(seam_points(&s, id), pts);
    let e = s.execute("document.export", &json!({"format": "svgcut", "size": "XXL"})).unwrap_err().to_string();
    assert!(e.contains("no size `XXL`"), "{e}");
}

#[test]
fn a_chart_size_exports_redrafted() {
    let mut s = skirt();
    let measured = s.execute("measure.info", &json!({})).unwrap();
    let page = |s: &mut Session, size: &str| -> f64 {
        let r = s.execute("document.export", &json!({"format": "svgcut", "size": size})).unwrap();
        let svg = String::from_utf8(vectorcraft_format::base64_decode(r["dataBase64"].as_str().unwrap()).unwrap()).unwrap();
        vectorcraft_svg::import(&svg).unwrap().artboards[0].rect.width()
    };
    let (ten, twelve, fourteen) = (page(&mut s, "10"), page(&mut s, "12"), page(&mut s, "14"));
    assert!(ten < twelve && twelve < fourteen, "{ten} {twelve} {fourteen}");
    // Drafting a size for the file leaves the document's own measurements alone.
    assert_eq!(s.execute("measure.info", &json!({})).unwrap(), measured);
}
