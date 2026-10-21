//! Pattern pieces as a DXF-AAMA / ASTM D6673 file, the format cutting rooms, grading and nesting
//! systems read. Written from the published descriptions of the standard: DH Patterns and Fit's
//! technical overview (<https://dorthehansen.com/wp-content/uploads/2014/10/ePattern-ASTM-Standard.pdf>)
//! and Patro's file format notes
//! (<https://fabricesalvaire.github.io/Patro/resources/file-format/dxf-astm.html>).
//!
//! One block per piece, inserted at the origin, its entities on layers named by number: 1 the
//! boundary (the cutting line) and the piece system text, 2 turn points and 3 curve points (of
//! layers 1 and 14), 4 notches (points carrying their angle, depth and width), 7 the grain line,
//! 8 internal lines, 14 the sew line, 15 annotation, and 84 and 87 the quality validation curves
//! (copies of the boundary and sew line). The style system text is on layer 1 at the origin,
//! outside every block. Coordinates are millimetres (`Units: METRIC`), y up.

use crate::DxfVersion;
use crate::writer::{Drawing, EntityStyle, Handle, Layer, UserBlock, fixed};

/// A point in millimetres, y up.
pub type Mm = [f64; 2];

/// A notch as ASTM describes it with a point on layer 4.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AamaNotch {
    /// On the boundary (the middle of a V notch's opening).
    pub at: Mm,
    /// The direction it is cut, into the piece: degrees counter-clockwise from +x.
    pub angle: f64,
    pub depth: f64,
    /// The opening's width at the boundary: 0 for a slit, more for a V notch.
    pub width: f64,
}

/// A pattern piece.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AamaPiece {
    /// Its name (unique in the file).
    pub name: String,
    /// How many to cut: right-side pieces and left-side (mirrored) pieces.
    pub quantity: (u32, u32),
    /// The material it is cut from; empty for none.
    pub material: String,
    /// The cutting line, closed (first point not repeated).
    pub boundary: Vec<Mm>,
    /// The sew line, closed; empty for none (the boundary is then the sew line too).
    pub sew_line: Vec<Mm>,
    pub notches: Vec<AamaNotch>,
    pub grain: Option<[Mm; 2]>,
    /// Lines drawn on the piece but not cut (placements, fold lines, darts).
    pub internal: Vec<Vec<Mm>>,
    /// Where the piece system text starts (inside the piece).
    pub text_at: Mm,
}

/// A pattern style: the pieces and what the style system text says about them.
#[derive(Clone, Debug, PartialEq)]
pub struct AamaStyle {
    pub name: String,
    /// `Author:`, as vendor;application;release.
    pub author: String,
    /// `dd-mm-yyyy`.
    pub date: String,
    /// `hh:mm`.
    pub time: String,
    pub sample_size: String,
    /// How far, in mm, curves may stray from the points written (`Curve Tolerance:`).
    pub tolerance: f64,
    /// R13 (ASTM D6673) or R12 (the older AAMA files some systems still want).
    pub version: DxfVersion,
    pub pieces: Vec<AamaPiece>,
}

/// Text height of the system text, mm.
const TEXT_MM: f64 = 3.5;
/// A turn of the outline sharper than this (degrees) is a turn point; a gentler one a curve point.
const TURN_DEGREES: f64 = 15.0;
/// Turns gentler than this are on a straight line: neither.
const STRAIGHT_DEGREES: f64 = 0.01;

/// The turn points and curve points of a closed polyline.
pub fn turn_and_curve_points(poly: &[Mm]) -> (Vec<Mm>, Vec<Mm>) {
    let n = poly.len();
    let (mut turns, mut curves) = (Vec::new(), Vec::new());
    if n < 3 {
        return (poly.to_vec(), curves);
    }
    for i in 0..n {
        let (a, b, c) = (poly[(i + n - 1) % n], poly[i], poly[(i + 1) % n]);
        let (u, v) = ([b[0] - a[0], b[1] - a[1]], [c[0] - b[0], c[1] - b[1]]);
        if u == [0.0, 0.0] || v == [0.0, 0.0] {
            continue;
        }
        let turn = (u[0] * v[1] - u[1] * v[0]).atan2(u[0] * v[0] + u[1] * v[1]).to_degrees().abs();
        if turn >= TURN_DEGREES {
            turns.push(b);
        } else if turn > STRAIGHT_DEGREES {
            curves.push(b);
        }
    }
    (turns, curves)
}

/// The style as a DXF-AAMA file.
pub fn export_aama(style: &AamaStyle) -> Result<String, String> {
    if !matches!(style.version, DxfVersion::R12 | DxfVersion::R13) {
        return Err("DXF-AAMA files are DXF R13 (ASTM D6673) or R12".into());
    }
    if style.pieces.is_empty() {
        return Err("no pattern pieces to export".into());
    }
    let v = style.version;
    let mut next = fixed::FIRST_FREE;
    let mut handle = || {
        let h = Handle(next);
        next += 1;
        h
    };
    let look = EntityStyle { aci: 7, ..Default::default() };
    let mut blocks = Vec::new();
    let mut names: Vec<String> = Vec::new();
    let mut extents = [f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY];
    let mut grow = |p: Mm| {
        extents = [extents[0].min(p[0]), extents[1].min(p[1]), extents[2].max(p[0]), extents[3].max(p[1])];
    };
    for piece in &style.pieces {
        if piece.boundary.len() < 3 {
            return Err(format!("the piece {:?} has no outline", piece.name));
        }
        let taken: Vec<&str> = names.iter().map(String::as_str).collect();
        let base = crate::scene::symbol_name(if piece.name.is_empty() { "PIECE" } else { &piece.name }, false);
        let name = crate::scene::unique(&base, &taken, 31);
        names.push(name.clone());
        let record = handle();
        let mut e = crate::writer::Codes::new(v);
        let text = |e: &mut crate::writer::Codes, h: Handle, layer: &str, at: Mm, s: &str| {
            e.entity("TEXT", h, record, layer, &look, "AcDbText");
            e.xyz(10, at[0], at[1]);
            e.num(40, TEXT_MM);
            e.str(1, s);
            if v.handles() {
                e.raw(100, "AcDbText");
            }
        };
        // The piece system text, one line under another.
        let mut lines = vec![format!("Piece Name: {}", piece.name), format!("Quantity: {},{}", piece.quantity.0, piece.quantity.1)];
        if !piece.material.is_empty() {
            lines.push(format!("Material: {}", piece.material));
        }
        for (i, l) in lines.iter().enumerate() {
            let at = [piece.text_at[0], piece.text_at[1] - i as f64 * TEXT_MM * 1.6];
            text(&mut e, handle(), "1", at, l);
        }
        let polyline = |e: &mut crate::writer::Codes, layer: &str, pts: &[Mm], closed: bool, handle: &mut dyn FnMut() -> Handle| {
            let h = handle();
            e.entity("POLYLINE", h, record, layer, &look, "AcDb2dPolyline");
            e.int(66, 1);
            e.xyz(10, 0.0, 0.0);
            e.int(70, i64::from(closed));
            for p in pts {
                let vh = handle();
                e.entity("VERTEX", vh, h, layer, &look, "AcDbVertex");
                if v.handles() {
                    e.raw(100, "AcDb2dVertex");
                }
                e.xyz(10, p[0], p[1]);
            }
            let end = handle();
            e.entity("SEQEND", end, h, layer, &look, "");
        };
        let point = |e: &mut crate::writer::Codes, h: Handle, layer: &str, p: Mm| {
            e.entity("POINT", h, record, layer, &look, "AcDbPoint");
            e.xyz(10, p[0], p[1]);
        };
        polyline(&mut e, "1", &piece.boundary, true, &mut handle);
        polyline(&mut e, "84", &piece.boundary, true, &mut handle);
        for p in &piece.boundary {
            grow(*p);
        }
        let (mut turns, mut curves) = turn_and_curve_points(&piece.boundary);
        if piece.sew_line.len() >= 3 {
            polyline(&mut e, "14", &piece.sew_line, true, &mut handle);
            polyline(&mut e, "87", &piece.sew_line, true, &mut handle);
            let (t, c) = turn_and_curve_points(&piece.sew_line);
            turns.extend(t);
            curves.extend(c);
        }
        for p in turns {
            point(&mut e, handle(), "2", p);
        }
        for p in curves {
            point(&mut e, handle(), "3", p);
        }
        for n in &piece.notches {
            let h = handle();
            e.entity("POINT", h, record, "4", &look, "AcDbPoint");
            // ASTM: the depth is the point's z, the width its thickness, the direction its angle.
            e.num(10, n.at[0]);
            e.num(20, n.at[1]);
            e.num(30, n.depth);
            if n.width > 0.0 {
                e.num(39, n.width);
            }
            e.num(50, n.angle);
        }
        if let Some([a, b]) = piece.grain {
            let h = handle();
            e.entity("LINE", h, record, "7", &look, "AcDbLine");
            e.xyz(10, a[0], a[1]);
            e.xyz(11, b[0], b[1]);
        }
        for l in &piece.internal {
            if l.len() >= 2 {
                polyline(&mut e, "8", l, false, &mut handle);
            }
        }
        let (begin, end) = (handle(), handle());
        blocks.push(UserBlock { name, record, begin, end, entities: e.into_string() });
    }
    // The style system text at the origin, then an insert of each piece.
    let mut e = crate::writer::Codes::new(v);
    let mut sst = vec![
        format!("Style Name: {}", style.name),
        format!("Creation Date: {}", style.date),
        format!("Creation Time: {}", style.time),
        format!("Author: {}", style.author),
    ];
    if !style.sample_size.is_empty() {
        sst.push(format!("Sample Size: {}", style.sample_size));
    }
    sst.extend(["Units: METRIC".to_string(), "ASTM/D13 Proposal 1 Version: D 6673-10".to_string()]);
    sst.push(format!("Curve Tolerance: {}", crate::writer::num(style.tolerance)));
    let count = sst.len();
    for (i, l) in sst.iter().enumerate() {
        let h = handle();
        e.entity("TEXT", h, fixed::MODEL_RECORD, "1", &look, "AcDbText");
        e.xyz(10, 0.0, (count - 1 - i) as f64 * TEXT_MM * 1.6);
        e.num(40, TEXT_MM);
        e.str(1, l);
        if v.handles() {
            e.raw(100, "AcDbText");
        }
    }
    for b in &blocks {
        let h = handle();
        e.entity("INSERT", h, fixed::MODEL_RECORD, "1", &look, "AcDbBlockReference");
        e.str(2, &b.name);
        e.xyz(10, 0.0, 0.0);
    }
    grow([0.0, 0.0]);
    let layers = ["1", "2", "3", "4", "7", "8", "14", "15", "84", "87"]
        .iter()
        .map(|n| Layer { name: (*n).into(), aci: 7, off: false, locked: false, plot: true })
        .collect();
    let drawing = Drawing {
        version: v,
        layers,
        linetypes: Vec::new(),
        styles: Vec::new(),
        images: Vec::new(),
        blocks,
        entities: e.into_string(),
        extents,
        limits: extents,
        insunits: 4,
        metric: true,
        next,
    };
    Ok(drawing.write())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square(x0: f64, y0: f64, side: f64) -> Vec<Mm> {
        vec![[x0, y0], [x0 + side, y0], [x0 + side, y0 + side], [x0, y0 + side]]
    }

    fn style(version: DxfVersion) -> AamaStyle {
        let piece = AamaPiece {
            name: "Test Square".into(),
            quantity: (1, 1),
            material: "Main".into(),
            boundary: square(0.0, 50.0, 120.0),
            sew_line: square(10.0, 60.0, 100.0),
            notches: vec![AamaNotch { at: [60.0, 50.0], angle: 90.0, depth: 6.0, width: 0.0 }],
            grain: Some([[60.0, 70.0], [60.0, 150.0]]),
            internal: vec![vec![[20.0, 110.0], [100.0, 110.0]]],
            text_at: [20.0, 100.0],
        };
        AamaStyle {
            name: "Test".into(),
            author: "SiteSentrix;PatternFoundry;0.1".into(),
            date: "21-10-2026".into(),
            time: "01:30".into(),
            sample_size: "M".into(),
            tolerance: 0.02,
            version,
            pieces: vec![piece],
        }
    }

    /// The `(code, value)` pairs of a DXF file.
    fn pairs(s: &str) -> Vec<(i32, String)> {
        let lines: Vec<&str> = s.lines().collect();
        lines.chunks(2).filter_map(|c| Some((c.first()?.trim().parse().ok()?, c.get(1)?.trim().to_string()))).collect()
    }

    /// The entities on `layer` inside the block `name`: (type, their pairs).
    fn block_entities(s: &str, name: &str, layer: &str) -> Vec<(String, Vec<(i32, String)>)> {
        let p = pairs(s);
        let mut out = Vec::new();
        let mut inside = false;
        let mut cur: Option<(String, Vec<(i32, String)>)> = None;
        for (i, (c, v)) in p.iter().enumerate() {
            if *c == 0 {
                if let Some(e) = cur.take()
                    && e.1.iter().any(|x| *x == (8, layer.to_string()))
                {
                    out.push(e);
                }
                if v == "BLOCK" {
                    inside = p[i + 1..].iter().take_while(|x| x.0 != 0).any(|x| *x == (2, name.to_string()));
                    continue;
                }
                if v == "ENDBLK" {
                    inside = false;
                }
                if inside {
                    cur = Some((v.clone(), Vec::new()));
                }
            } else if let Some(e) = &mut cur {
                e.1.push((*c, v.clone()));
            }
        }
        out
    }

    fn coords(e: &[(i32, String)], x: i32) -> Vec<f64> {
        e.iter().filter(|p| p.0 == x).filter_map(|p| p.1.parse().ok()).collect()
    }

    #[test]
    fn a_piece_is_a_block_with_its_cutting_line_sew_line_and_marks_on_their_layers() {
        for version in [DxfVersion::R13, DxfVersion::R12] {
            let s = export_aama(&style(version)).unwrap();
            assert!(s.contains(version.acadver()));
            // True scale: the 10 cm sew line is 100 mm across in the file, the cutting line 120.
            let verts = |layer: &str| -> (Vec<f64>, Vec<f64>) {
                let v: Vec<_> = block_entities(&s, "TEST_SQUARE", layer).into_iter().filter(|e| e.0 == "VERTEX").collect();
                (v.iter().flat_map(|e| coords(&e.1, 10)).collect(), v.iter().flat_map(|e| coords(&e.1, 20)).collect())
            };
            let span = |xs: &[f64]| xs.iter().cloned().fold(f64::NEG_INFINITY, f64::max) - xs.iter().cloned().fold(f64::INFINITY, f64::min);
            let (sx, sy) = verts("14");
            assert_eq!((sx.len(), span(&sx), span(&sy)), (4, 100.0, 100.0), "{version:?}");
            let (bx, by) = verts("1");
            assert_eq!((span(&bx), span(&by)), (120.0, 120.0));
            // The validation curves copy them.
            assert_eq!(verts("84"), (bx, by));
            assert_eq!(verts("87"), (sx, sy));
            // Four turn points on each line, no curve points on a square.
            assert_eq!(block_entities(&s, "TEST_SQUARE", "2").len(), 8);
            assert!(block_entities(&s, "TEST_SQUARE", "3").is_empty());
            // The notch: a point on the cutting line, 6 deep, cut at 90°.
            let notch = &block_entities(&s, "TEST_SQUARE", "4")[0];
            assert_eq!(notch.0, "POINT");
            assert_eq!((coords(&notch.1, 10), coords(&notch.1, 30), coords(&notch.1, 50)), (vec![60.0], vec![6.0], vec![90.0]));
            assert_eq!(block_entities(&s, "TEST_SQUARE", "7")[0].0, "LINE");
            assert_eq!(block_entities(&s, "TEST_SQUARE", "8").iter().filter(|e| e.0 == "POLYLINE").count(), 1);
            // The piece system text in the block, the style system text outside it.
            let pst: Vec<String> = block_entities(&s, "TEST_SQUARE", "1")
                .iter()
                .filter(|e| e.0 == "TEXT")
                .flat_map(|e| e.1.iter().filter(|p| p.0 == 1).map(|p| p.1.clone()).collect::<Vec<_>>())
                .collect();
            assert_eq!(pst, ["Piece Name: Test Square", "Quantity: 1,1", "Material: Main"]);
            for line in ["Style Name: Test", "Units: METRIC", "Creation Date: 21-10-2026", "Sample Size: M", "Curve Tolerance: 0.02"] {
                assert!(s.contains(&format!("\n{line}\n")), "{line}");
            }
            // The block is inserted once, at the origin.
            let p = pairs(&s);
            let inserts = p.iter().enumerate().filter(|(_, x)| **x == (0, "INSERT".into())).count();
            assert_eq!(inserts, 1);
            assert!(s.ends_with("EOF\n"));
        }
    }

    #[test]
    fn names_are_unique_block_names_and_bad_styles_are_refused() {
        let mut st = style(DxfVersion::R13);
        let mut second = st.pieces[0].clone();
        second.name = "test square".into();
        st.pieces.push(second);
        let s = export_aama(&st).unwrap();
        assert!(s.contains("\nTEST_SQUARE\n") && s.contains("\nTEST_SQUARE_2\n"));
        assert!(export_aama(&AamaStyle { version: DxfVersion::R2018, ..style(DxfVersion::R13) }).is_err());
        assert!(export_aama(&AamaStyle { pieces: vec![], ..style(DxfVersion::R13) }).is_err());
    }

    #[test]
    fn corners_are_turn_points_and_bends_curve_points() {
        // A quarter circle in 10° steps closed by two straight edges.
        let mut poly: Vec<Mm> = (0..=9)
            .map(|k| {
                let a = (k as f64 * 10.0).to_radians();
                [100.0 * a.cos(), 100.0 * a.sin()]
            })
            .collect();
        poly.push([0.0, 0.0]);
        let (turns, curves) = turn_and_curve_points(&poly);
        assert_eq!(turns.len(), 3);
        assert_eq!(curves.len(), 8);
    }
}
