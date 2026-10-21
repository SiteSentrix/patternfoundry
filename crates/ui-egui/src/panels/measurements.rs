//! Measurements panel (PF-3): the document's measurements (in mm) and formulas, the size chart's
//! sizes, and whether exported files may carry the measured values. Everything it shows comes
//! from `measure.info`; every change goes through `measure.*`.
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
    let mut share = info["share"].as_bool().unwrap_or(false);
    if ui
        .checkbox(&mut share, tl!("Share in exported files"))
        .on_hover_text(tl!("Let PDF and SVG files saved with Preserve Editing keep the measured values"))
        .changed()
    {
        run(app, "measure.set", json!({ "share": share }));
    }
}
