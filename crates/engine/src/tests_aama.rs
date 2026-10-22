//! DXF-AAMA export (PF-7.1, `document.export {format: aama}`): pattern pieces as an ASTM
//! D6673 file, at true scale.

use pf_core::mm_to_pt;
use serde_json::{Value, json};

use super::*;

/// A 10 cm square made a piece, with a 10 mm allowance and a notch in the middle of its top edge.
fn square_piece(cut: u32, mirror: bool) -> Session {
    let mut s = Session::new();
    s.execute("file.new", &json!({"width": 800, "height": 800})).unwrap();
    let side = mm_to_pt(100.0);
    s.execute("shape.rectangle", &json!({"x": 40, "y": 40, "width": side, "height": side})).unwrap();
    s.execute("piece.make", &json!({"name": "Front", "allowance": 10})).unwrap();
    s.execute("piece.set", &json!({"cut": cut, "mirror": mirror, "fabric": "Main"})).unwrap();
    s.execute("piece.addNotch", &json!({"edge": 0, "at": 0.5})).unwrap();
    s
}

fn export(s: &mut Session, p: Value) -> String {
    let r = s.execute("document.export", &p).unwrap();
    // DXF lines end in CR LF; the checks below read lines.
    String::from_utf8(vectorcraft_format::base64_decode(r["dataBase64"].as_str().unwrap()).unwrap()).unwrap().replace("\r\n", "\n")
}

/// The `(code, value)` pairs of a DXF file.
fn pairs(s: &str) -> Vec<(i32, String)> {
    let lines: Vec<&str> = s.lines().collect();
    lines.chunks(2).filter_map(|c| Some((c.first()?.trim().parse().ok()?, c.get(1)?.trim().to_string()))).collect()
}

/// Each entity of type `kind` on `layer`: its pairs.
fn entities(s: &str, kind: &str, layer: &str) -> Vec<Vec<(i32, String)>> {
    let p = pairs(s);
    let mut out = Vec::new();
    for (i, x) in p.iter().enumerate() {
        if *x == (0, kind.to_string()) {
            let body: Vec<(i32, String)> = p[i + 1..].iter().take_while(|y| y.0 != 0).cloned().collect();
            if body.iter().any(|y| *y == (8, layer.to_string())) {
                out.push(body);
            }
        }
    }
    out
}

fn values(e: &[(i32, String)], code: i32) -> Vec<f64> {
    e.iter().filter(|p| p.0 == code).filter_map(|p| p.1.parse().ok()).collect()
}

fn span(xs: &[f64]) -> f64 {
    xs.iter().cloned().fold(f64::NEG_INFINITY, f64::max) - xs.iter().cloned().fold(f64::INFINITY, f64::min)
}

#[test]
fn the_10_cm_square_is_100_mm_on_the_sew_line_and_120_on_the_cutting_line() {
    let mut s = square_piece(1, false);
    for version in ["R13", "R12"] {
        let dxf = export(&mut s, json!({"format": "aama", "version": version, "styleName": "Test", "sampleSize": "M"}));
        assert!(dxf.contains(if version == "R13" { "AC1012" } else { "AC1009" }));
        // True scale: the seam line (layer 14) of the 10 cm square measures exactly 100 mm, the
        // cutting line (layer 1) 120 with 10 mm either side.
        let verts = |layer: &str| entities(&dxf, "VERTEX", layer);
        let xs = |layer: &str| verts(layer).iter().flat_map(|e| values(e, 10)).collect::<Vec<_>>();
        let ys = |layer: &str| verts(layer).iter().flat_map(|e| values(e, 20)).collect::<Vec<_>>();
        assert!((span(&xs("14")) - 100.0).abs() < 1e-6 && (span(&ys("14")) - 100.0).abs() < 1e-6, "{:?}", xs("14"));
        assert!((span(&xs("1")) - 120.0).abs() < 1e-6 && (span(&ys("1")) - 120.0).abs() < 1e-6);
        assert_eq!((xs("84"), xs("87")), (xs("1"), xs("14")));
        // The notch, on the cutting line at the middle of the top edge, 6 mm deep, cut straight
        // down (y up: -90°).
        let notch = &entities(&dxf, "POINT", "4")[0];
        let top = ys("1").iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        assert!(
            (values(notch, 20)[0] - top).abs() < 1e-6
                && (values(notch, 10)[0] - (xs("1").iter().cloned().fold(f64::INFINITY, f64::min) + 60.0)).abs() < 1e-6
        );
        assert_eq!((values(notch, 30), values(notch, 50)), (vec![6.0], vec![-90.0]));
        assert_eq!(entities(&dxf, "LINE", "7").len(), 1);
        // The texts.
        for line in ["Piece Name: Front", "Quantity: 1,0", "Material: Main", "Style Name: Test", "Sample Size: M", "Units: METRIC"] {
            assert!(dxf.contains(&format!("\n{line}\n")), "{line}");
        }
    }
}

#[test]
fn mirrored_pairs_are_right_and_left_pieces_and_a_document_without_pieces_is_refused() {
    let mut s = square_piece(2, true);
    assert!(export(&mut s, json!({"format": "aama"})).contains("\nQuantity: 1,1\n"));
    assert!(s.execute("document.export", &json!({"format": "aama", "version": "2018"})).is_err());
    let mut t = Session::new();
    t.execute("file.new", &json!({"width": 800, "height": 800})).unwrap();
    t.execute("shape.rectangle", &json!({"x": 0, "y": 0, "width": 100, "height": 100})).unwrap();
    let e = t.execute("document.export", &json!({"format": "aama"})).unwrap_err().to_string();
    assert!(e.contains("pattern pieces"), "{e}");
}

#[test]
fn unnamed_pieces_go_by_their_block_part_and_every_name_is_once_in_the_file() {
    let mut s = Session::new();
    s.execute("file.new", &json!({"width": 1400, "height": 1000})).unwrap();
    s.execute("measure.set", &json!({"values": {"waist": 680, "hip": 940, "waist_to_hip": 200, "skirt_length": 600}})).unwrap();
    s.execute("block.skirt", &json!({"origin": [0, 0]})).unwrap();
    s.execute("select.all", &json!({})).unwrap();
    s.execute("piece.make", &json!({})).unwrap();
    let side = mm_to_pt(100.0);
    for _ in 0..2 {
        s.execute("shape.rectangle", &json!({"x": 900, "y": 0, "width": side, "height": side})).unwrap();
        s.execute("piece.make", &json!({"name": "Pocket"})).unwrap();
    }
    let dxf = export(&mut s, json!({"format": "aama"}));
    for name in ["Back Skirt Block", "Front Skirt Block", "Pocket", "Pocket 2"] {
        assert!(dxf.contains(&format!("\nPiece Name: {name}\n")), "{name}");
    }
}

#[test]
fn a_piece_cut_on_the_fold_is_the_half_with_its_mirror_line_on_layer_6() {
    let mut s = square_piece(1, false);
    // The left side (edge 3) on the fold.
    s.execute("piece.set", &json!({"fold": 3})).unwrap();
    let dxf = export(&mut s, json!({"format": "aama"}));
    let verts = |layer: &str| entities(&dxf, "VERTEX", layer);
    let xs = |layer: &str| verts(layer).iter().flat_map(|e| values(e, 10)).collect::<Vec<_>>();
    // The half: 110 mm across (no allowance on the fold), the seam line still 100.
    assert!((span(&xs("1")) - 110.0).abs() < 1e-6 && (span(&xs("14")) - 100.0).abs() < 1e-6, "{:?}", xs("1"));
    // The mirror line: down the fold, where the cutting line and the seam line meet, 100 mm long.
    let m = entities(&dxf, "LINE", "6");
    assert_eq!(m.len(), 1);
    let (x, y) = ([values(&m[0], 10)[0], values(&m[0], 11)[0]], [values(&m[0], 20)[0], values(&m[0], 21)[0]]);
    let left = xs("1").iter().cloned().fold(f64::INFINITY, f64::min);
    assert!((x[0] - left).abs() < 1e-6 && (x[1] - left).abs() < 1e-6 && ((y[0] - y[1]).abs() - 100.0).abs() < 1e-6, "{x:?} {y:?}");
    // Without a fold there is none.
    s.execute("piece.set", &json!({"fold": null})).unwrap();
    assert!(entities(&export(&mut s, json!({"format": "aama"})), "LINE", "6").is_empty());
}
