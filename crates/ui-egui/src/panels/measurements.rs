//! Measurements panel (PF-3): the document's measurements (in mm) and formulas, the size chart's
//! sizes, whether exported files may carry the measured values, and My Measurements: the sets kept
//! on this computer to load into any pattern (PF-3.4). Everything it shows comes from
//! `measure.info` and `measure.sets`; every change goes through `measure.*`.
//!
//! The values are personal data: the panel shows them, and nothing here logs them.

use egui::Ui;
use serde_json::{Value, json};

use super::{pstate, set_pstate};
use crate::VectorcraftApp;
use crate::theme::Tokens;
use crate::widgets;

#[derive(Clone, Default)]
struct Cache {
    key: Option<(u64, u64)>,
    info: Value,
}

fn fetch(app: &mut VectorcraftApp, ctx: &egui::Context) -> Option<Value> {
    let st = app.session.active()?;
    let key = Some((st.uid, st.revision));
    let mut c: Cache = pstate(ctx, "measure-cache");
    if c.key != key {
        c.info = app.session.execute("measure.info", &json!({})).unwrap_or_default();
        c.key = key;
        set_pstate(ctx, "measure-cache", c.clone());
    }
    Some(c.info)
}

/// The saved sets (`measure.sets`), read from their folder again only when the document changes
/// or the panel saved or deleted one.
fn saved_sets(app: &mut VectorcraftApp, ctx: &egui::Context) -> Value {
    let key = app.session.active().map(|st| (st.uid, st.revision));
    let mut c: Cache = pstate(ctx, "measure-sets");
    if c.key != key || c.info.is_null() {
        c.info = app.session.execute("measure.sets", &json!({})).unwrap_or_default();
        c.key = key;
        set_pstate(ctx, "measure-sets", c.clone());
    }
    c.info
}

/// Run `id` with `p`, then read the saved sets again.
fn run_on_sets(app: &mut VectorcraftApp, ctx: &egui::Context, id: &str, p: Value) {
    run(app, id, p);
    set_pstate(ctx, "measure-sets", Cache::default());
}

fn run(app: &mut VectorcraftApp, id: &str, p: Value) {
    if let Err(e) = app.run(id, p) {
        app.status(e);
    }
}

pub fn show(app: &mut VectorcraftApp, ui: &mut Ui) {
    let Some(info) = fetch(app, ui.ctx()) else {
        widgets::dim_label(ui, tl!("No document"));
        return;
    };
    let t = Tokens::get(ui.ctx());
    let values = info["values"].as_object().cloned().unwrap_or_default();
    let formulas = info["formulas"].as_object().cloned().unwrap_or_default();
    if values.is_empty() && formulas.is_empty() {
        widgets::dim_label(ui, tl!("No measurements yet"));
    }
    egui::ScrollArea::vertical().max_height(320.0).show(ui, |ui| {
        for (name, v) in &values {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(name).size(12.0).color(t.text));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.small_button("×").on_hover_text(tl!("Remove")).clicked() {
                        run(app, "measure.set", json!({ "values": { name.as_str(): null } }));
                    }
                    let mut mm = v.as_f64().unwrap_or_default();
                    let r = ui.add(egui::DragValue::new(&mut mm).range(0.0..=10_000.0).speed(1.0).max_decimals(1).suffix(" mm"));
                    if r.drag_stopped() || (r.changed() && !r.dragged()) {
                        run(app, "measure.set", json!({ "values": { name.as_str(): mm } }));
                    }
                });
            });
        }
        if !formulas.is_empty() {
            widgets::subheader(ui, tl!("Formulas"));
        }
        for (name, f) in &formulas {
            let value = info["computed"][name.as_str()].as_f64().map_or_else(|| "—".to_string(), |v| format!("{v} mm"));
            let shown = format!("{name} = {}  →  {value}", f.as_str().unwrap_or_default());
            ui.label(egui::RichText::new(shown).size(12.0).color(t.text));
            if let Some(e) = info["errors"][name.as_str()].as_str() {
                widgets::dim_label(ui, e);
            }
        }
    });
    widgets::divider(ui);
    // Add a measurement: a name and its value.
    let mut add: (String, f64) = pstate(ui.ctx(), "measure-add");
    ui.horizontal(|ui| {
        ui.add(egui::TextEdit::singleline(&mut add.0).hint_text(tl!("Name")).desired_width(110.0));
        ui.add(egui::DragValue::new(&mut add.1).range(0.0..=10_000.0).suffix(" mm"));
        if ui.button(tl!("Add Measurement")).clicked() && !add.0.trim().is_empty() {
            run(app, "measure.set", json!({ "values": { add.0.trim(): add.1 } }));
            add = Default::default();
        }
    });
    set_pstate(ui.ctx(), "measure-add", add);
    if let Some(sizes) = info["chart"]["sizes"].as_array() {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(tl!("Size chart")).size(12.0).color(t.text_dim));
            let current = info["size"].as_str().unwrap_or("");
            egui::ComboBox::from_id_salt("measure-size").selected_text(current).show_ui(ui, |ui| {
                for s in sizes.iter().filter_map(Value::as_str) {
                    if ui.selectable_label(current.ends_with(s), s).clicked() {
                        run(app, "measure.useSize", json!({ "size": s }));
                    }
                }
            });
        });
    }
    // Whose proportions the avatar fills in the rest from (PF-10.11).
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(tl!("Figure")).size(12.0).color(t.text_dim))
            .on_hover_text(tl!("The avatar takes what the measurements don't give from a survey's women's or men's proportions"));
        let current = info["figure"].as_str().unwrap_or("female");
        let label = |f: &str| if f == "male" { tl!("Male") } else { tl!("Female") };
        egui::ComboBox::from_id_salt("measure-figure").selected_text(label(current)).show_ui(ui, |ui| {
            for f in ["female", "male"] {
                if ui.selectable_label(current == f, label(f)).clicked() && current != f {
                    run(app, "measure.set", json!({ "figure": f }));
                }
            }
        });
    });
    let mut share = info["share"].as_bool().unwrap_or(false);
    if ui
        .checkbox(&mut share, tl!("Share in exported files"))
        .on_hover_text(tl!("Let PDF and SVG files saved with Preserve Editing keep the measured values"))
        .changed()
    {
        run(app, "measure.set", json!({ "share": share }));
    }
    my_measurements(app, ui, &info);
}

/// My Measurements: the sets kept on this computer, each with Load and Delete, and the
/// document's set saved under a name.
fn my_measurements(app: &mut VectorcraftApp, ui: &mut Ui, info: &Value) {
    let sets = saved_sets(app, ui.ctx());
    if sets["store"].as_bool() != Some(true) {
        return;
    }
    let t = Tokens::get(ui.ctx());
    widgets::divider(ui);
    widgets::subheader(ui, tl!("My Measurements"));
    widgets::dim_label(ui, tl!("Kept on this computer only, to load into any pattern"));
    let ctx = ui.ctx().clone();
    for set in sets["sets"].as_array().into_iter().flatten() {
        let name = set["name"].as_str().unwrap_or_default().to_string();
        ui.horizontal(|ui| {
            // The name and how many measurements the set holds.
            let count = set["count"].as_u64().unwrap_or_default();
            ui.label(egui::RichText::new(format!("{name} ({count})")).size(12.0).color(t.text));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.small_button("×").on_hover_text(tl!("Delete from this computer")).clicked() {
                    run_on_sets(app, &ctx, "measure.deleteSet", json!({ "name": name }));
                }
                if ui.small_button(tl!("Load")).on_hover_text(tl!("Use these measurements in this pattern")).clicked() {
                    run_on_sets(app, &ctx, "measure.loadSet", json!({ "name": name }));
                }
            });
        });
    }
    let has = info["values"].as_object().is_some_and(|v| !v.is_empty()) || info["formulas"].as_object().is_some_and(|f| !f.is_empty());
    if has {
        let mut name: String = pstate(ui.ctx(), "measure-save-name");
        if name.is_empty() {
            name = info["name"].as_str().unwrap_or_default().to_string();
        }
        ui.horizontal(|ui| {
            ui.add(egui::TextEdit::singleline(&mut name).hint_text(tl!("Whose (Me)")).desired_width(110.0));
            if ui.button(tl!("Save to My Measurements")).clicked() && !name.trim().is_empty() {
                run_on_sets(app, &ctx, "measure.saveSet", json!({ "name": name.trim() }));
            }
        });
        set_pstate(ui.ctx(), "measure-save-name", name);
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use vectorcraft_engine::Session;

    use crate::VectorcraftApp;
    use crate::tests_labels::painted_text;

    #[test]
    fn my_measurements_list_the_saved_sets_when_there_is_a_folder() {
        let mut app = VectorcraftApp::new(Session::new(), Default::default());
        app.run("file.new", json!({})).unwrap();
        // Made-up measurements.
        app.run("measure.set", json!({"name": "Me", "values": {"bust": 880}})).unwrap();
        // No folder to keep sets in (the web, a test): no section.
        assert!(!painted_text(&mut app, super::show).contains("My Measurements"));
        let dir = vectorcraft_testkit::temp_dir("panel-my-measurements").join("Measurements");
        let _ = std::fs::remove_dir_all(&dir);
        app.session.attach_measurement_store(dir.clone());
        let text = painted_text(&mut app, super::show);
        assert!(text.contains("My Measurements") && text.contains("Save to My Measurements"), "{text}");
        app.run("measure.saveSet", json!({})).unwrap();
        let text = painted_text(&mut app, super::show);
        assert!(text.contains("Me (1)") && text.contains("Load"), "{text}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
