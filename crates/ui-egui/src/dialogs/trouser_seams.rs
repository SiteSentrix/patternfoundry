//! Pattern › Check Trouser Seams (`ui.checkTrouserSeams`, PF-4.6): the trouser block's front
//! walked against its back, run by run (`design.trouserSeams`), told in a message with The Shapes
//! of Fabric tutorial's advice on lines that don't match.

use serde_json::{Value, json};

use crate::VectorcraftApp;

/// A run's name, as `design.trouserSeams` gives it, in the interface language.
fn run_name(seam: &str) -> &'static str {
    match seam {
        "inseam, knee to hem" => tl!("Inseam, knee to hem"),
        "inseam, crotch to knee" => tl!("Inseam, crotch to knee"),
        "side seam, knee to hem" => tl!("Side seam, knee to hem"),
        _ => tl!("Side seam, waist to knee"),
    }
}

/// Walk the drafted trouser block's seams and tell the result.
pub fn check(app: &mut VectorcraftApp, p: &Value) -> Result<Value, String> {
    let params = if p.is_object() { p.clone() } else { json!({}) };
    let r = app.session.execute("design.trouserSeams", &params).map_err(|e| e.to_string())?;
    let n = |v: &Value| v.as_f64().map(|v| format!("{v}")).unwrap_or_default();
    let lines: Vec<String> = r["runs"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|run| {
            crate::i18n::fmt(
                tl!("{seam}: front {front} mm, back {back} mm ({difference} mm)"),
                &[
                    ("seam", run_name(run["seam"].as_str().unwrap_or_default())),
                    ("front", &n(&run["front"])),
                    ("back", &n(&run["back"])),
                    ("difference", &n(&run["difference"])),
                ],
            )
        })
        .collect();
    let advice = tl!(
        "Walk them on the pattern too: where a line doesn't run on smoothly from front to back, find the point half way between the two and correct both lines. The back inseam is more curved, so it is often longer: lowering the back crotch curve slightly to match is fine."
    );
    super::confirm::tell(app, tl!("Trouser Seams"), &format!("{}\n{advice}", lines.join("\n")));
    Ok(r)
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use vectorcraft_engine::Session;

    use crate::VectorcraftApp;

    #[test]
    fn the_check_tells_each_run_front_and_back() {
        let mut app = VectorcraftApp::new(Session::new(), Default::default());
        app.run("file.new", json!({"width": 1400, "height": 1400})).unwrap();
        assert!(app.run("ui.checkTrouserSeams", json!({})).is_err(), "no trouser block yet");
        let m = json!({"hip": 960, "waist": 740, "waist_to_hip": 200, "crotch_depth": 260, "waist_to_knee": 600, "waist_to_ankle": 1040, "trouser_hem": 460});
        app.run("measure.set", json!({"values": m})).unwrap();
        app.run("block.trousers", json!({})).unwrap();
        let r = app.run("ui.checkTrouserSeams", json!({})).unwrap();
        let d = app.ui.dialog.as_ref().unwrap();
        assert_eq!(d.kind, super::super::confirm::MESSAGE);
        let detail = d.str("detail");
        let first = &r["runs"][0];
        assert!(
            detail.starts_with(&format!(
                "Inseam, knee to hem: front {} mm, back {} mm",
                first["front"].as_f64().unwrap(),
                first["back"].as_f64().unwrap()
            )),
            "{detail}"
        );
        assert_eq!(detail.lines().count(), 5, "{detail}");
    }
}
