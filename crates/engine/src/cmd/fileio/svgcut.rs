//! SVG for cutting (`document.export {format: svgcut}`, PF-7.2): the document's pattern pieces as
//! an SVG for laser cutters, cutting machines and print shops, sized in millimetres so it is
//! true to scale in any reader. Each piece's cutting line is a closed red hairline path in the
//! `cut` group, its notches short blue cuts in `notches`, and (with `marks`, the default) its
//! seam line (dashed), grainline, the lines drawn inside it and its name and cut count in black in
//! `draw`, which a cutter can be told to skip or to draw with a pen.

use serde_json::Value;
use vectorcraft_doc::Document;

use super::super::*;
use super::{Encoded, FormatOption};
use pf_core::pt_to_mm;

const C: &str = "document.export";
/// White space round the pieces, mm.
const MARGIN_MM: f64 = 10.0;
/// The type size of the piece names, mm.
const TEXT_MM: f64 = 5.0;

/// The cutting SVG's options `document.formats` lists.
pub(super) const OPTIONS: &[FormatOption] = &[
    FormatOption {
        name: "marks",
        ty: "boolean",
        default: "true",
        description: "also the seam lines, grainlines, lines inside the pieces and their names (group `draw`, not cut)",
    },
    FormatOption {
        name: "size",
        ty: "string",
        default: "\"\"",
        description: "write the pieces graded to this size of the size chart or size range (default: as drawn)",
    },
];

/// A number in mm, to a thousandth, without trailing zeros.
fn n(v: f64) -> String {
    let s = format!("{:.3}", if v.is_finite() { v } else { 0.0 });
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" { "0".into() } else { s.into() }
}

fn escape(s: &str) -> String {
    s.chars()
        .filter(|c| !c.is_control())
        .map(|c| match c {
            '&' => "&amp;".into(),
            '<' => "&lt;".into(),
            '>' => "&gt;".into(),
            '"' => "&quot;".into(),
            c => c.to_string(),
        })
        .collect()
}

/// Path data through `pts` (mm).
fn d(pts: &[[f64; 2]], closed: bool) -> String {
    let mut s = String::new();
    for (i, p) in pts.iter().enumerate() {
        s.push_str(if i == 0 { "M" } else { " L" });
        s.push_str(&format!("{} {}", n(p[0]), n(p[1])));
    }
    if closed {
        s.push_str(" Z");
    }
    s
}

pub(super) fn encode(doc: &Document, p: &Value) -> Result<Encoded> {
    let marks = p.get("marks").and_then(Value::as_bool).unwrap_or(true);
    // The pieces in points, y down (as the document is).
    let pieces = super::aama::pieces(doc)?;
    if pieces.is_empty() {
        return Err(bad(C, "no pattern pieces to export: make the outlines pattern pieces first (Pattern › Make Pattern Piece)"));
    }
    let (mut x0, mut y0, mut x1, mut y1) = (f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
    for q in pieces.iter().flat_map(|p| &p.boundary) {
        (x0, y0, x1, y1) = (x0.min(q[0]), y0.min(q[1]), x1.max(q[0]), y1.max(q[1]));
    }
    let at = |q: [f64; 2]| [pt_to_mm(q[0] - x0) + MARGIN_MM, pt_to_mm(q[1] - y0) + MARGIN_MM];
    let (w, h) = (pt_to_mm(x1 - x0) + 2.0 * MARGIN_MM, pt_to_mm(y1 - y0) + 2.0 * MARGIN_MM);
    let mut out = String::new();
    out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}mm\" height=\"{}mm\" viewBox=\"0 0 {} {}\">\n",
        n(w),
        n(h),
        n(w),
        n(h)
    ));
    out.push_str("  <title>Pattern pieces for cutting</title>\n");
    out.push_str("  <g id=\"cut\" fill=\"none\" stroke=\"#ff0000\" stroke-width=\"0.1\">\n");
    for (i, piece) in pieces.iter().enumerate() {
        let pts: Vec<[f64; 2]> = piece.boundary.iter().map(|q| at(*q)).collect();
        out.push_str(&format!("    <path id=\"cut-{}\" data-name=\"{}\" d=\"{}\"/>\n", i + 1, escape(&piece.name), d(&pts, true)));
    }
    out.push_str("  </g>\n");
    let notches: Vec<String> = pieces
        .iter()
        .flat_map(|piece| &piece.notches)
        .map(|nt| {
            // The notch's angle is counter-clockwise with y up; the document's y runs down.
            let (s, c) = nt.angle.to_radians().sin_cos();
            let into = [c, -s];
            let a = at(nt.at);
            let (depth, half) = (pt_to_mm(nt.depth), pt_to_mm(nt.width) / 2.0);
            let tip = [a[0] + into[0] * depth, a[1] + into[1] * depth];
            if half > 0.0 {
                let side = [-into[1] * half, into[0] * half];
                d(&[[a[0] + side[0], a[1] + side[1]], tip, [a[0] - side[0], a[1] - side[1]]], false)
            } else {
                d(&[a, tip], false)
            }
        })
        .collect();
    if !notches.is_empty() {
        out.push_str("  <g id=\"notches\" fill=\"none\" stroke=\"#0000ff\" stroke-width=\"0.1\">\n");
        for nd in notches {
            out.push_str(&format!("    <path d=\"{nd}\"/>\n"));
        }
        out.push_str("  </g>\n");
    }
    if marks {
        out.push_str("  <g id=\"draw\" fill=\"none\" stroke=\"#000000\" stroke-width=\"0.2\">\n");
        for piece in &pieces {
            if piece.sew_line.len() >= 3 {
                let pts: Vec<[f64; 2]> = piece.sew_line.iter().map(|q| at(*q)).collect();
                out.push_str(&format!("    <path d=\"{}\" stroke-dasharray=\"2 1\"/>\n", d(&pts, true)));
            }
            if let Some([a, b]) = piece.grain {
                out.push_str(&format!("    <path d=\"{}\"/>\n", d(&[at(a), at(b)], false)));
            }
            for l in piece.internal.iter().filter(|l| l.len() >= 2) {
                let pts: Vec<[f64; 2]> = l.iter().map(|q| at(*q)).collect();
                out.push_str(&format!("    <path d=\"{}\"/>\n", d(&pts, false)));
            }
            let t = at(piece.text_at);
            let cut = piece.quantity.0 + piece.quantity.1;
            let mut second = format!("Cut {cut}");
            if piece.quantity.1 > 0 {
                second.push_str(" (mirrored pairs)");
            }
            if !piece.material.is_empty() {
                second.push_str(&format!(" · {}", piece.material));
            }
            out.push_str(&format!(
                "    <text x=\"{}\" y=\"{}\" font-family=\"sans-serif\" font-size=\"{}\" fill=\"#000000\" stroke=\"none\"><tspan x=\"{}\">{}</tspan><tspan x=\"{}\" dy=\"{}\">{}</tspan></text>\n",
                n(t[0] + 3.0),
                n(t[1]),
                n(TEXT_MM),
                n(t[0] + 3.0),
                escape(&piece.name),
                n(t[0] + 3.0),
                n(TEXT_MM * 1.3),
                escape(&second)
            ));
        }
        out.push_str("  </g>\n");
    }
    out.push_str("</svg>\n");
    Ok(Encoded::one(out.into_bytes()))
}
