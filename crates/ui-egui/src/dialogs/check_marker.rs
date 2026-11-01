//! Pattern › Check Marker (`ui.checkMarker`, PF-9.3): after a marker's pieces are moved by hand,
//! `marker.check` draws the fabric to the last piece again and brings its caption up to date;
//! the result, with any pieces that overlap or lie off the fabric, is told in a message.

use serde_json::{Value, json};

use crate::VectorcraftApp;

/// Check the marker the selection is on (or the document's only one) and tell the result.
pub fn check(app: &mut VectorcraftApp, p: &Value) -> Result<Value, String> {
    let params = if p.is_object() { p.clone() } else { json!({}) };
    let r = app.session.execute("marker.check", &params).map_err(|e| e.to_string())?;
    let n = |v: &Value| v.as_f64().map(|v| format!("{v}")).unwrap_or_default();
    let mut lines = vec![crate::i18n::fmt(
        tl!("{length} cm long, {efficiency}% of the fabric used"),
        &[("length", &n(&json!(r["lengthMm"].as_f64().unwrap_or_default() / 10.0))), ("efficiency", &n(&r["efficiency"]))],
    )];
    for pair in r["overlaps"].as_array().into_iter().flatten() {
        let name = |i: usize| pair[i].as_str().unwrap_or_default().to_string();
        lines.push(crate::i18n::fmt(tl!("{a} overlaps {b}"), &[("a", &name(0)), ("b", &name(1))]));
    }
    for name in r["offFabric"].as_array().into_iter().flatten() {
        lines.push(crate::i18n::fmt(tl!("{piece} is off the fabric"), &[("piece", name.as_str().unwrap_or_default())]));
    }
    if r["overlaps"].as_array().is_none_or(|v| v.is_empty()) && r["offFabric"].as_array().is_none_or(|v| v.is_empty()) {
        lines.push(tl!("No pieces overlap, and all are on the fabric.").to_string());
    }
    super::confirm::tell(app, tl!("Check Marker"), &lines.join("\n"));
    Ok(r)
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use vectorcraft_engine::Session;

    use crate::VectorcraftApp;

    #[test]
    fn the_check_tells_the_length_and_what_is_wrong() {
        let mut app = VectorcraftApp::new(Session::new(), Default::default());
        app.run("file.new", json!({"width": 2000, "height": 2000})).unwrap();
        assert!(app.run("ui.checkMarker", json!({})).is_err(), "no marker yet");
        let r = app.run("shape.rectangle", json!({"x": 0, "y": 0, "width": 850.4, "height": 1417.3})).unwrap();
        let piece = app.run("piece.make", json!({"ids": [r["id"]]})).unwrap()["ids"][0].clone();
        app.run("piece.set", json!({"id": piece, "cut": 2})).unwrap();
        app.run("marker.make", json!({"width": "110 cm", "gap": 0, "origin": [0, 2000]})).unwrap();
        let r = app.run("ui.checkMarker", json!({})).unwrap();
        let d = app.ui.dialog.as_ref().unwrap();
        assert_eq!(d.kind, super::super::confirm::MESSAGE);
        let detail = d.str("detail");
        let cm = r["lengthMm"].as_f64().unwrap() / 10.0;
        assert!(detail.starts_with(&format!("{cm} cm long, ")) && detail.ends_with("No pieces overlap, and all are on the fabric."), "{detail}");
    }
}
