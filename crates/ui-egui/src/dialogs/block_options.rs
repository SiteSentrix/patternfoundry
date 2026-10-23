//! Pattern › Block Options… (`ui.blockOptions`, PF-4.5): the selected drafted block's ease and
//! darts (`block.options`), each with the block's default beside it and a button back to it; OK
//! changes them (`block.setOptions`), and every part of the block is drawn again.
//!
//! Fields: `__id` (the block part), `__options` (the options as `block.options` lists them) and one
//! field per option, its name: mm, or null for the block's default.

use serde_json::{Map, Value, json};

use super::DialogSpec;
use crate::state::Dialog;
use crate::theme::Tokens;
use crate::{VectorcraftApp, widgets};

/// The dialog kind of Block Options.
pub const KIND: &str = "blockOptions";

/// Width of the label column.
const LABEL_W: f32 = 150.0;

pub(super) const SPEC: DialogSpec = DialogSpec { heading: |_| tl!("Block Options").into(), body, confirm, min_width: 380.0, ..DialogSpec::FORM };

/// Open Block Options on the selected block part (or the block part `id` in `p`).
pub fn open(app: &mut VectorcraftApp, p: &Value) -> Result<Value, String> {
    let params = match p.get("id") {
        Some(id) => json!({ "id": id }),
        None => json!({}),
    };
    let info = app.session.execute("block.options", &params).map_err(|e| e.to_string())?;
    let mut fields = Map::new();
    for o in info["options"].as_array().into_iter().flatten() {
        if let Some(name) = o["name"].as_str() {
            fields.insert(name.into(), o["value"].clone());
        }
    }
    fields.insert("__id".into(), info["id"].clone());
    fields.insert("__options".into(), info["options"].clone());
    app.ui.dialog = Some(Dialog::new(KIND, Value::Object(fields)));
    Ok(info)
}

/// An option's label.
fn label(name: &str) -> &'static str {
    match name {
        "ease" => tl!("Ease"),
        "waist_ease" => tl!("Waist ease"),
        "bust_dart" => tl!("Bust dart"),
        "shoulder_dart" => tl!("Shoulder dart"),
        "back_dart" => tl!("Back dart"),
        "front_dart" => tl!("Front dart"),
        "crown_ease" => tl!("Crown ease"),
        "back_dart_length" => tl!("Back dart length"),
        "front_dart_length" => tl!("Front dart length"),
        _ => tl!("Option"),
    }
}

fn body(_app: &mut VectorcraftApp, ui: &mut egui::Ui, d: &mut Dialog) -> bool {
    let t = Tokens::get(ui.ctx());
    let options = d.fields.get("__options").and_then(Value::as_array).cloned().unwrap_or_default();
    for o in &options {
        let Some(name) = o["name"].as_str() else { continue };
        let (min, max) = (o["min"].as_f64().unwrap_or(0.0), o["max"].as_f64().unwrap_or(1000.0));
        let set = d.fields.get(name).and_then(Value::as_f64);
        let default = o["default"].as_f64();
        widgets::label_row(ui, label(name), LABEL_W, |ui| {
            let mut mm = set.or(default).unwrap_or(min);
            let r = ui
                .add(egui::DragValue::new(&mut mm).range(min..=max).speed(0.5).max_decimals(1).suffix(" mm"))
                .on_hover_text(o["what"].as_str().unwrap_or_default());
            if r.changed() {
                d.fields.insert(name.into(), json!(mm));
            }
            match (set, default) {
                (Some(_), _) => {
                    if ui.small_button("↺").on_hover_text(tl!("Back to the block's default")).clicked() {
                        d.fields.insert(name.into(), Value::Null);
                    }
                }
                (None, None) => {
                    ui.label(egui::RichText::new(tl!("from the measurements")).size(11.0).color(t.text_dim));
                }
                (None, Some(_)) => {}
            }
        });
    }
    false
}

fn confirm(app: &mut VectorcraftApp, d: &Dialog) -> Result<Value, String> {
    let options: Map<String, Value> = d
        .fields
        .get("__options")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|o| o["name"].as_str())
        .map(|name| (name.to_string(), d.fields.get(name).cloned().unwrap_or(Value::Null)))
        .collect();
    let id = d.fields.get("__id").cloned().unwrap_or(Value::Null);
    let r = app.session.execute("block.setOptions", &json!({ "id": id, "options": options })).map_err(|e| e.to_string())?;
    app.ui.dialog = None;
    Ok(r)
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use vectorcraft_engine::Session;

    use crate::VectorcraftApp;

    #[test]
    fn block_options_set_the_ease_and_go_back_to_the_default() {
        let mut app = VectorcraftApp::new(Session::new(), Default::default());
        app.run("file.new", json!({"width": 1400, "height": 1000})).unwrap();
        app.run("measure.set", json!({"values": {"top_arm": 380, "sleeve_length": 640}})).unwrap();
        assert!(app.run("ui.blockOptions", json!({})).is_err(), "nothing drafted, nothing selected");
        let id = app.run("block.sleeve", json!({"origin": [0, 0]})).unwrap()["ids"][0].as_u64().unwrap();
        app.run("ui.blockOptions", json!({})).unwrap();
        let d = app.ui.dialog.as_ref().unwrap();
        assert_eq!((d.kind.as_str(), d.fields["ease"].clone()), ("blockOptions", json!(null)));
        super::super::tests_export::frame(&mut app);
        app.ui.dialog.as_mut().unwrap().fields.insert("ease".into(), json!(30));
        super::super::confirm(&mut app).unwrap();
        assert!(app.ui.dialog.is_none());
        let info = app.run("block.options", json!({"id": id})).unwrap();
        assert_eq!(info["options"][0]["value"], json!(30.0));
        // Back to the default.
        app.run("ui.blockOptions", json!({"id": id})).unwrap();
        app.ui.dialog.as_mut().unwrap().fields.insert("ease".into(), json!(null));
        super::super::confirm(&mut app).unwrap();
        assert_eq!(app.run("block.options", json!({"id": id})).unwrap()["options"][0]["value"], json!(null));
    }
}
