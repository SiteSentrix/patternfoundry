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
    assert!(t.execute("grade.nest", &json!({})).unwrap_err().to_string().contains("size chart"));
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
