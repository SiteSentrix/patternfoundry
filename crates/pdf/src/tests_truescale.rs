//! True scale (PF-2, CLAUDE.md): a pattern printed at 100% measures exactly. Each print and export
//! path is checked by reading the PDF back: the page is the paper's size, a 10 cm square drawn in
//! the document is 10 cm on the page, and with the sewing-pattern marks every page carries a 10 cm
//! and a 1 in test square.

use serde_json::json;
use vectorcraft_color::{Color, Paint};
use vectorcraft_doc::{Appearance, Artboard, Document, Node, NodeKind};
use vectorcraft_geom::{Rect, shapes};

use crate::*;

/// 10 cm in points.
const CM10: f64 = 7200.0 / 25.4;

/// A `w` × `h` document with a filled 10 cm square at `(x, y)`.
fn doc(w: f64, h: f64, x: f64, y: f64) -> Document {
    let mut d = Document::new(w, h);
    let layer = d.default_layer().unwrap();
    let id = d.alloc_id();
    let fill = Appearance::basic(Paint::solid(Color::rgb(0.8, 0.1, 0.1)), Paint::None, 0.0);
    d.insert(Some(layer), 0, Node::path(id, shapes::rectangle(Rect::new(x, y, x + CM10, y + CM10)), fill)).unwrap();
    d
}

fn print_pdf(d: &Document, v: serde_json::Value) -> Vec<u8> {
    let settings: PrintSettings = serde_json::from_value(v).unwrap();
    print(d, &PrintOptions { settings, created: Some(1_791_200_000), uncompressed: true, ..Default::default() }).unwrap().bytes
}

/// A page read back: its size, and the sizes of the paths on it (filled or stroked) that lie on it.
type Page = ((f64, f64), Vec<(f64, f64)>);

fn pages(bytes: &[u8]) -> Vec<Page> {
    let back = import(bytes).unwrap();
    back.artboards
        .iter()
        .map(|ab| {
            let r = ab.rect;
            let mut sizes = vec![];
            back.walk(|n| {
                if let Some(b) = n.path_data().and_then(|p| p.bounds())
                    && b.intersect(r).area() > 0.0
                    && r.contains_rect(b)
                {
                    sizes.push((b.width(), b.height()));
                }
            });
            ((r.width(), r.height()), sizes)
        })
        .collect()
}

fn has_square(sizes: &[(f64, f64)], side: f64) -> bool {
    sizes.iter().any(|(w, h)| (w - side).abs() < 0.01 && (h - side).abs() < 0.01)
}

fn close(a: (f64, f64), b: (f64, f64)) -> bool {
    (a.0 - b.0).abs() < 0.01 && (a.1 - b.1).abs() < 0.01
}

#[test]
fn a_pattern_printed_at_100_percent_measures_exactly() {
    let d = doc(612.0, 792.0, 100.0, 100.0);
    let p = pages(&print_pdf(&d, json!({"scaling": "none", "autoRotate": false})));
    assert_eq!(p.len(), 1);
    assert!(close(p[0].0, (612.0, 792.0)), "Letter: {:?}", p[0].0);
    assert!(has_square(&p[0].1, CM10), "the 10 cm square is 10 cm: {:?}", p[0].1);
    // No test squares unless asked for.
    assert!(!has_square(&p[0].1, 72.0));
}

#[test]
fn every_tile_of_a_tiled_pattern_carries_both_test_squares() {
    // 1500 × 1100 pt over Letter with half-inch margins and a quarter-inch overlap.
    let d = doc(1500.0, 1100.0, 40.0, 40.0);
    let set = json!({"scaling": "tileFull", "margin": 36, "overlap": 18, "autoRotate": false, "patternMarks": true});
    let p = pages(&print_pdf(&d, set));
    assert!(p.len() >= 4, "{} pages", p.len());
    for (i, (size, sizes)) in p.iter().enumerate() {
        assert!(close(*size, (612.0, 792.0)), "page {i}: {size:?}");
        assert!(has_square(sizes, CM10), "page {i} has its 10 cm square: {sizes:?}");
        assert!(has_square(sizes, 72.0), "page {i} has its 1 in square: {sizes:?}");
    }
    // The art's own 10 cm square, on the first tile, is 10 cm too (twice: it and the test square).
    assert!(p[0].1.iter().filter(|(w, h)| (w - CM10).abs() < 0.01 && (h - CM10).abs() < 0.01).count() >= 2);
}

#[test]
fn tiles_are_named_by_row_and_column() {
    let d = doc(1500.0, 1100.0, 40.0, 40.0);
    let settings: PrintSettings =
        serde_json::from_value(json!({"scaling": "tileFull", "margin": 36, "overlap": 18, "autoRotate": false, "patternMarks": true})).unwrap();
    let pv = preview(&d, &settings).unwrap();
    let (cols, rows) = (pv.tiles[0].columns, pv.tiles[0].rows);
    let plan = plan(&d, &PrintOptions { settings, ..Default::default() }).unwrap();
    let mut names = vec![];
    for s in &plan.sheets {
        let Some(marks) = &s.marks else { continue };
        marks.walk(&mut |n: &Node| {
            if let NodeKind::Text(t) = &n.kind {
                let text: String = t.runs.iter().map(|r| r.text.as_str()).collect();
                if text.contains('×') {
                    names.push(text);
                }
            }
        });
    }
    assert_eq!(names.len(), cols * rows);
    assert_eq!(names.first().map(String::as_str), Some(format!("A1  ({rows} × {cols})").as_str()));
    assert_eq!(names.last().map(String::as_str), Some(format!("{}{cols}  ({rows} × {cols})", pf_core::tiling::row_name(rows - 1)).as_str()));
}

#[test]
fn copy_shop_sheets_print_whole_at_100_percent() {
    // A0 (841 × 1189 mm) and the 36 in roll (36 × 48 in), the pattern on one sheet.
    let d = doc(2000.0, 3000.0, 500.0, 500.0);
    for (media, size) in
        [("a0", (841.0 * 72.0 / 25.4, 1189.0 * 72.0 / 25.4)), ("roll36", (2592.0, 3456.0)), ("a1", (594.0 * 72.0 / 25.4, 841.0 * 72.0 / 25.4))]
    {
        let p = pages(&print_pdf(&d, json!({"media": media, "scaling": "none", "autoRotate": false, "patternMarks": true})));
        assert_eq!(p.len(), 1, "{media}");
        assert!(close(p[0].0, size), "{media}: {:?}", p[0].0);
        assert!(has_square(&p[0].1, CM10) && has_square(&p[0].1, 72.0), "{media}: {:?}", p[0].1);
    }
}

#[test]
fn exported_pdf_pages_are_their_artboards_at_100_percent() {
    let mut d = doc(1000.0, 1000.0, 100.0, 100.0);
    // A second artboard the size of A4.
    let a4 = (210.0 * 72.0 / 25.4, 297.0 * 72.0 / 25.4);
    d.artboards.push(Artboard {
        id: 2,
        name: "A4".into(),
        rect: Rect::new(1200.0, 0.0, 1200.0 + a4.0, a4.1),
        show_center_mark: false,
        show_cross_hairs: false,
    });
    let id = d.alloc_id();
    let layer = d.default_layer().unwrap();
    let fill = Appearance::basic(Paint::solid(Color::rgb(0.1, 0.1, 0.8)), Paint::None, 0.0);
    d.insert(Some(layer), 0, Node::path(id, shapes::rectangle(Rect::new(1300.0, 50.0, 1300.0 + CM10, 50.0 + CM10)), fill)).unwrap();
    let bytes = export(&d, &PdfOptions::default()).unwrap();
    let info = info(&bytes, None).unwrap();
    assert_eq!(info.pages.len(), 2);
    assert!(close((info.pages[0].width, info.pages[0].height), (1000.0, 1000.0)));
    assert!(close((info.pages[1].width, info.pages[1].height), a4));
    let p = pages(&bytes);
    assert!(has_square(&p[0].1, CM10) && has_square(&p[1].1, CM10), "{p:?}");
}

#[test]
fn full_page_tiles_that_leave_gaps_are_warned_about() {
    let d = doc(1500.0, 1100.0, 40.0, 40.0);
    let warned = |v: serde_json::Value| {
        let settings: PrintSettings = serde_json::from_value(v).unwrap();
        plan(&d, &PrintOptions { settings, ..Default::default() }).unwrap().warnings.iter().any(|w| w.contains("Tile Full Pages"))
    };
    assert!(warned(json!({"scaling": "tileFull", "margin": 36, "overlap": 18, "patternMarks": true})));
    assert!(!warned(json!({"scaling": "tileFull", "margin": 36, "overlap": 80, "patternMarks": true})), "overlap covers both margins");
    assert!(!warned(json!({"scaling": "tileImageable", "margin": 36, "overlap": 18, "patternMarks": true})));
    assert!(!warned(json!({"scaling": "tileFull", "margin": 36, "overlap": 18})), "only with the pattern marks");
}
