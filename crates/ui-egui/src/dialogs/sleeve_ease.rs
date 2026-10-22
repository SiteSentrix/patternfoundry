//! Pattern › Check Sleeve Ease (`ui.checkSleeveEase`): the sleeve's crown against the armhole it
//! is set into (`design.sleeveEase`), told in a message: the lengths, the ease and whether it is
//! in the range a set-in sleeve takes in, with the `sleeve_crown` that redrafts the sleeve block
//! to the middle of it.

use serde_json::{Value, json};

use crate::VectorcraftApp;

/// Check the drafted sleeve and bodice blocks (or the seams in `p`, as `design.sleeveEase` takes
/// them) and tell the result.
pub fn check(app: &mut VectorcraftApp, p: &Value) -> Result<Value, String> {
    let params = if p.is_object() { p.clone() } else { json!({}) };
    let r = app.session.execute("design.sleeveEase", &params).map_err(|e| e.to_string())?;
    let n = |k: &str| r[k].as_f64().map(|v| format!("{v}")).unwrap_or_default();
    let lengths = crate::i18n::fmt(
        tl!("Crown {crown} mm, armhole {armhole} mm (back {back}, front {front}): the crown takes in {ease} mm ({percent}%)."),
        &[
            ("crown", &n("crown")),
            ("armhole", &n("armhole")),
            ("back", &n("back")),
            ("front", &n("front")),
            ("ease", &n("ease")),
            ("percent", &n("percent")),
        ],
    );
    let range = [("min", n("min")), ("max", n("max")), ("wanted", n("sleeveCrown"))];
    let args: Vec<(&str, &str)> = range.iter().map(|(k, v)| (*k, v.as_str())).collect();
    let verdict = match r["fits"].as_str() {
        Some("tight") => crate::i18n::fmt(
            tl!(
                "Less than the {min} to {max} mm it should take in: the sleeve may pull across the cap. Set sleeve_crown to {wanted} mm in Measurements to redraft the sleeve block with the ease in the middle."
            ),
            &args,
        ),
        Some("full") => crate::i18n::fmt(
            tl!(
                "More than the {min} to {max} mm it should take in: the cap may pucker when it is set in. Set sleeve_crown to {wanted} mm in Measurements to redraft the sleeve block with the ease in the middle."
            ),
            &args,
        ),
        _ => crate::i18n::fmt(tl!("Within the {min} to {max} mm it should take in."), &args),
    };
    super::confirm::tell(app, tl!("Sleeve Ease"), &format!("{lengths}\n{verdict}"));
    Ok(r)
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use vectorcraft_engine::Session;

    use crate::VectorcraftApp;

    #[test]
    fn the_check_tells_the_lengths_and_what_to_set() {
        let mut app = VectorcraftApp::new(Session::new(), Default::default());
        // Without blocks it says what is missing.
        assert!(app.run("ui.checkSleeveEase", json!({})).is_err());
        app.run("file.new", json!({"width": 1400, "height": 1000})).unwrap();
        // Made-up measurements, not from any size chart.
        let m = json!({"bust": 880, "waist": 680, "nape_to_waist": 410, "back_width": 345, "chest": 325, "shoulder": 122, "neck": 370, "armscye_depth": 210, "top_arm": 280, "sleeve_length": 590});
        app.run("measure.set", json!({"values": m})).unwrap();
        app.run("block.bodice", json!({"origin": [0, 0]})).unwrap();
        app.run("block.sleeve", json!({"origin": [2000, 0]})).unwrap();
        // Drafted from the top arm table alone, the crown is about the armhole's length: no ease.
        let r = app.run("ui.checkSleeveEase", json!({})).unwrap();
        assert_eq!(r["fits"], "tight");
        let d = app.ui.dialog.as_ref().unwrap();
        assert_eq!(d.kind, super::super::confirm::MESSAGE);
        let detail = d.str("detail");
        let n = |k: &str| r[k].as_f64().unwrap().to_string();
        assert!(detail.starts_with(&format!("Crown {} mm, armhole {} mm", n("crown"), n("armhole"))), "{detail}");
        assert!(detail.contains(&format!("Set sleeve_crown to {} mm", n("sleeveCrown"))), "{detail}");
    }
}
