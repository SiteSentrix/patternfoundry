//! PDF for projector (PF-7.3, `document.export {format: projector}`): the pattern pieces on one
//! page at 100%, with calibration grids and a 10 cm test square as PDF layers.

use pf_core::{mm_to_pt, pt_to_mm};
use serde_json::{Value, json};
use vectorcraft_doc::{Document, Node, NodeKind};
use vectorcraft_geom::Rect;

use super::*;

/// A 10 cm square made a piece with a 10 mm allowance and a notch in its top edge.
fn square_piece() -> Session {
    let mut s = Session::new();
    s.execute("file.new", &json!({"width": 800, "height": 800})).unwrap();
    let side = mm_to_pt(100.0);
    s.execute("shape.rectangle", &json!({"x": 40, "y": 40, "width": side, "height": side})).unwrap();
    s.execute("piece.make", &json!({"name": "Front", "allowance": 10})).unwrap();
    s.execute("piece.addNotch", &json!({"edge": 0, "at": 0.5})).unwrap();
    s
}

/// The exported PDF, read back.
fn export(s: &mut Session, p: Value) -> Document {
    let r = s.execute("document.export", &p).unwrap();
    let pdf = vectorcraft_format::base64_decode(r["dataBase64"].as_str().unwrap()).unwrap();
    vectorcraft_pdf::import(&pdf).unwrap()
}

fn layer<'a>(doc: &'a Document, name: &str) -> &'a Node {
    doc.layers.iter().find(|l| l.name.as_deref() == Some(name)).unwrap_or_else(|| panic!("no layer {name}"))
}

/// The bounds and stroke weight of each closed path under `n`.
fn closed_paths(n: &Node) -> Vec<(Rect, f64)> {
    let mut out = Vec::new();
    n.walk(&mut |c: &Node| {
        if let NodeKind::Path { path, .. } = &c.kind
            && path.subpaths.len() == 1
            && path.subpaths[0].closed
            && let Some(b) = c.geometric_bounds()
        {
            out.push((b, c.appearance.stroke().map_or(0.0, |s| s.width)));
        }
    });
    out
}

fn mm(r: Rect) -> [f64; 4] {
    [r.x0, r.y0, r.width(), r.height()].map(|v| (pt_to_mm(v) * 100.0).round() / 100.0)
}

#[test]
fn the_10_cm_test_square_measures_10_cm_and_the_pieces_are_at_100_percent() {
    let mut s = square_piece();
    let doc = export(&mut s, json!({"format": "projector"}));
    // One page: the pieces' 120 mm, 25 mm round them and a 150 mm band above for the square.
    let page = doc.artboards[0].rect;
    assert_eq!(doc.artboards.len(), 1);
    assert_eq!([pt_to_mm(page.width()), pt_to_mm(page.height())].map(|v| v.round()), [170.0, 295.0], "{page:?}");
    // The layers, bottom first; the inch grid starts hidden.
    let layers: Vec<(String, bool)> = doc.layers.iter().map(|l| (l.name.clone().unwrap_or_default(), l.visible)).collect();
    let names = ["10 cm grid", "4 in grid", "Test square", "Pieces"];
    assert_eq!(layers, names.iter().zip([true, false, true, true]).map(|(n, v)| (n.to_string(), v)).collect::<Vec<_>>());
    // The test square: 10 cm by 10 cm, 25 mm from the page's top left corner.
    let square = closed_paths(layer(&doc, "Test square"));
    assert_eq!(square.iter().map(|(b, _)| mm(*b)).collect::<Vec<_>>(), [[25.0, 25.0, 100.0, 100.0]]);
    // The cutting line, 3 pt thick, and the seam line 10 mm inside it.
    let pieces = closed_paths(layer(&doc, "Pieces"));
    let cut = pieces.iter().find(|(_, w)| (*w - 3.0).abs() < 1e-6).unwrap();
    assert_eq!(mm(cut.0), [25.0, 150.0, 120.0, 120.0]);
    assert!(pieces.iter().any(|(b, w)| (*w - 1.0).abs() < 1e-6 && mm(*b) == [35.0, 160.0, 100.0, 100.0]), "{pieces:?}");
}

#[test]
fn inverted_and_mirrored_pages() {
    let mut s = square_piece();
    let doc = export(&mut s, json!({"format": "projector", "invert": true, "mirror": true, "margin": 50, "lineWidth": 4}));
    assert_eq!(doc.layers[0].name.as_deref(), Some("Background"));
    // Mirrored: the test square on the right, still 10 cm.
    let page = pt_to_mm(doc.artboards[0].rect.width()).round();
    assert_eq!(page, 220.0);
    let square = closed_paths(layer(&doc, "Test square"));
    assert_eq!(square.iter().map(|(b, _)| mm(*b)).collect::<Vec<_>>(), [[70.0, 50.0, 100.0, 100.0]]);
    let pieces = closed_paths(layer(&doc, "Pieces"));
    let cut = pieces.iter().find(|(_, w)| (*w - 4.0).abs() < 1e-6).unwrap();
    assert_eq!(mm(cut.0), [50.0, 200.0, 120.0, 120.0]);
    // White on black.
    let white = |n: &Node| n.appearance.stroke().is_some_and(|s| s.paint == vectorcraft_color::Paint::solid(vectorcraft_color::Color::WHITE));
    let mut found = false;
    layer(&doc, "Pieces").walk(&mut |c: &Node| found |= matches!(c.kind, NodeKind::Path { .. }) && white(c));
    assert!(found);
}

#[test]
fn bad_options_and_no_pieces_are_refused() {
    let mut s = square_piece();
    let e = s.execute("document.export", &json!({"format": "projector", "lineWidth": 0})).unwrap_err().to_string();
    assert!(e.contains("lineWidth"), "{e}");
    let mut t = Session::new();
    t.execute("file.new", &json!({"width": 800, "height": 800})).unwrap();
    t.execute("shape.rectangle", &json!({"x": 0, "y": 0, "width": 100, "height": 100})).unwrap();
    assert!(t.execute("document.export", &json!({"format": "projector"})).unwrap_err().to_string().contains("pattern pieces"));
}

#[test]
fn touching_pieces_are_set_apart_unless_kept_as_drawn() {
    // Two 10 cm squares side by side: their 10 mm allowances overlap by 20 mm as drawn.
    let mut s = Session::new();
    s.execute("file.new", &json!({"width": 1000, "height": 800})).unwrap();
    let side = mm_to_pt(100.0);
    for x in [40.0, 40.0 + side] {
        s.execute("shape.rectangle", &json!({"x": x, "y": 40, "width": side, "height": side})).unwrap();
        s.execute("piece.make", &json!({"allowance": 10})).unwrap();
    }
    let cuts = |doc: &Document| {
        let mut v: Vec<[f64; 4]> =
            closed_paths(layer(doc, "Pieces")).into_iter().filter(|(_, w)| (*w - 3.0).abs() < 1e-6).map(|(b, _)| mm(b)).collect();
        v.sort_by(|a, b| a[0].total_cmp(&b[0]));
        v
    };
    let row = export(&mut s, json!({"format": "projector"}));
    assert_eq!(cuts(&row), [[25.0, 150.0, 120.0, 120.0], [170.0, 150.0, 120.0, 120.0]]);
    let drawn = export(&mut s, json!({"format": "projector", "layout": "asDrawn"}));
    assert_eq!(cuts(&drawn), [[25.0, 150.0, 120.0, 120.0], [125.0, 150.0, 120.0, 120.0]]);
    assert!(s.execute("document.export", &json!({"format": "projector", "layout": "grid"})).unwrap_err().to_string().contains("layout"));
}

#[test]
fn a_piece_cut_on_the_fold_is_projected_whole_or_as_the_half_with_its_fold() {
    let mut s = square_piece();
    s.execute("piece.set", &json!({"fold": 3})).unwrap();
    // Opened out: one cutting line 220 mm across (twice 100 + 10), the seam line 200, and the
    // notch on both halves; the test square still 10 cm.
    let doc = export(&mut s, json!({"format": "projector"}));
    assert_eq!(closed_paths(layer(&doc, "Test square")).iter().map(|(b, _)| mm(*b)).collect::<Vec<_>>(), [[25.0, 25.0, 100.0, 100.0]]);
    let pieces = closed_paths(layer(&doc, "Pieces"));
    let cuts: Vec<[f64; 4]> = pieces.iter().filter(|(_, w)| (*w - 3.0).abs() < 1e-6).map(|(b, _)| mm(*b)).collect();
    assert_eq!(cuts, [[25.0, 150.0, 220.0, 120.0]]);
    assert!(pieces.iter().any(|(b, w)| (*w - 1.0).abs() < 1e-6 && mm(*b) == [35.0, 160.0, 200.0, 100.0]), "{pieces:?}");
    let open = |doc: &Document| {
        let mut v = Vec::new();
        layer(doc, "Pieces").walk(&mut |c: &Node| {
            if let NodeKind::Path { path, .. } = &c.kind
                && path.subpaths.iter().all(|sp| !sp.closed)
                && let Some(st) = c.appearance.stroke()
                && (st.width - 3.0).abs() < 1e-6
            {
                v.push((st.dash.is_some(), c.geometric_bounds().unwrap()));
            }
        });
        v
    };
    assert_eq!(open(&doc).iter().filter(|(dashed, _)| !dashed).count(), 2, "a notch on each half");
    // As the half, for folded fabric: 110 mm across, the fold dashed down its side.
    let doc = export(&mut s, json!({"format": "projector", "openFolds": false}));
    let cuts: Vec<[f64; 4]> = closed_paths(layer(&doc, "Pieces")).iter().filter(|(_, w)| (*w - 3.0).abs() < 1e-6).map(|(b, _)| mm(*b)).collect();
    assert_eq!(cuts, [[25.0, 150.0, 110.0, 120.0]]);
    let fold: Vec<Rect> = open(&doc).into_iter().filter(|(dashed, _)| *dashed).map(|(_, b)| b).collect();
    assert_eq!(fold.len(), 1);
    assert!(mm(fold[0])[0] == 25.0 && mm(fold[0])[2] == 0.0 && mm(fold[0])[3] == 100.0, "{:?}", mm(fold[0]));
}
