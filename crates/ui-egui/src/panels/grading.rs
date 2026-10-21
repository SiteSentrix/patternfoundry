//! Grading panel (PF-6): the document's size range and base size, the selected piece's grade
//! rules point by point, and the graded nest. Everything it shows comes from `grade.info`; every
//! change goes through `grade.*`.

use egui::Ui;
use serde_json::{Value, json};

use super::{pstate, set_pstate};
use crate::VectorcraftApp;
use crate::theme::Tokens;
use crate::widgets;

/// What the panel shows, fetched again only when the document changes: (document uid, revision).
#[derive(Clone, Default)]
struct Cache {
    key: Option<(u64, u64)>,
    info: Option<Value>,
}

fn fetch(app: &mut VectorcraftApp, ctx: &egui::Context) -> Option<Cache> {
    let st = app.session.active()?;
    let key = Some((st.uid, st.revision));
    let mut c: Cache = pstate(ctx, "grading-cache");
    if c.key != key {
        c.info = app.session.execute("grade.info", &json!({})).ok();
        c.key = key;
        set_pstate(ctx, "grading-cache", c.clone());
    }
    Some(c)
}

/// Run `id` with `p`; a refusal goes to the status bar.
fn run(app: &mut VectorcraftApp, id: &str, p: Value) {
    if let Err(e) = app.run(id, p) {
        app.status(e);
    }
}

/// The size range typed as "S, M, L" and the base size; Set Sizes sends them.
fn size_range(app: &mut VectorcraftApp, ui: &mut Ui, info: Option<&Value>) {
    let t = Tokens::get(ui.ctx());
    let sizes: Vec<String> =
        info.and_then(|i| i["sizes"].as_array()).map(|a| a.iter().filter_map(|s| s.as_str().map(String::from)).collect()).unwrap_or_default();
    let base = info.and_then(|i| i["base"].as_str()).unwrap_or("").to_string();
    let current = sizes.join(", ");
    let mut buf: (String, String, String) = pstate(ui.ctx(), "grading-sizes");
    // Follow the document until the user types.
    if buf.2 != current {
        buf = (current.clone(), base.clone(), current.clone());
    }
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(tl!("Sizes")).size(12.0).color(t.text_dim));
        ui.add(egui::TextEdit::singleline(&mut buf.0).hint_text("S, M, L").desired_width(f32::INFINITY));
    });
    let typed: Vec<String> = buf.0.split([',', ';']).map(str::trim).filter(|s| !s.is_empty()).map(String::from).collect();
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(tl!("Base size")).size(12.0).color(t.text_dim));
        egui::ComboBox::from_id_salt("grading-base").selected_text(buf.1.as_str()).show_ui(ui, |ui| {
            for s in &typed {
                if ui.selectable_label(buf.1 == *s, s.as_str()).clicked() {
                    buf.1 = s.clone();
                }
            }
        });
        if ui.button(tl!("Set Size Range")).clicked() && !typed.is_empty() {
            // Without a base chosen yet, the middle size.
            let base = if typed.contains(&buf.1) { buf.1.clone() } else { typed[typed.len() / 2].clone() };
            run(app, "grade.setSizes", json!({ "sizes": typed, "base": base }));
        }
    });
    set_pstate(ui.ctx(), "grading-sizes", buf);
}

/// The selected piece's points, each with its step rule (mm a size).
fn rules(app: &mut VectorcraftApp, ui: &mut Ui, info: &Value) {
    let t = Tokens::get(ui.ctx());
    let Some(id) = info["id"].as_u64() else { return };
    let Some(base) = info["base"].as_str() else {
        widgets::dim_label(ui, tl!("Name the sizes, smallest first, to give the points grade rules"));
        return;
    };
    let count = info["moves"][base].as_array().map_or(0, Vec::len);
    widgets::subheader(ui, tl!("Grade rules (mm a size)"));
    egui::ScrollArea::vertical().max_height(280.0).show(ui, |ui| {
        for anchor in 0..count {
            let rule = info["rules"].as_array().and_then(|r| r.iter().find(|r| r["anchor"].as_u64() == Some(anchor as u64)));
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(format!("{anchor}")).size(12.0).color(if rule.is_some() { t.text } else { t.text_dim }));
                match rule {
                    // A rule naming each size is edited by command; the panel shows it is there.
                    Some(r) if r.get("sizes").is_some() => {
                        widgets::dim_label(ui, tl!("per size"));
                    }
                    _ => {
                        let step = rule.and_then(|r| r["step"].as_array());
                        let mut m = [0, 1].map(|i| step.and_then(|s| s.get(i)).and_then(Value::as_f64).unwrap_or(0.0));
                        let mut changed = false;
                        for (axis, v) in ["x", "y"].iter().zip(m.iter_mut()) {
                            let r = ui.add(egui::DragValue::new(v).range(-500.0..=500.0).speed(0.1).max_decimals(2).prefix(format!("{axis} ")));
                            changed |= r.drag_stopped() || (r.changed() && !r.dragged());
                        }
                        if changed {
                            run(app, "grade.setRule", json!({ "id": id, "anchor": anchor, "step": m }));
                        }
                    }
                }
                if rule.is_some() && ui.small_button("×").on_hover_text(tl!("Remove")).clicked() {
                    run(app, "grade.setRule", json!({ "id": id, "anchor": anchor, "clear": true }));
                }
            });
        }
    });
    if info["rules"].as_array().is_some_and(|r| !r.is_empty()) {
        copy_to(app, ui, id);
    }
}

/// Copy the selected piece's rules to another piece, mirrored for the opposite half when asked.
fn copy_to(app: &mut VectorcraftApp, ui: &mut Ui, id: u64) {
    let pieces: Vec<(u64, String)> = app
        .session
        .execute("piece.list", &json!({}))
        .ok()
        .and_then(|v| v["pieces"].as_array().cloned())
        .unwrap_or_default()
        .iter()
        .filter_map(|p| Some((p["id"].as_u64()?, p["name"].as_str().unwrap_or("").to_string())))
        .filter(|(p, _)| *p != id)
        .collect();
    if pieces.is_empty() {
        return;
    }
    let name = |p: &(u64, String)| if p.1.trim().is_empty() { format!("{} {}", tl!("Pattern piece"), p.0) } else { p.1.clone() };
    let mut st: (u64, bool) = pstate(ui.ctx(), "grading-copy");
    if !pieces.iter().any(|p| p.0 == st.0) {
        st.0 = pieces[0].0;
    }
    ui.horizontal(|ui| {
        let shown = pieces.iter().find(|p| p.0 == st.0).map(name).unwrap_or_default();
        egui::ComboBox::from_id_salt("grading-copy-to").selected_text(shown).show_ui(ui, |ui| {
            for p in &pieces {
                if ui.selectable_label(st.0 == p.0, name(p)).clicked() {
                    st.0 = p.0;
                }
            }
        });
        ui.checkbox(&mut st.1, tl!("Mirrored")).on_hover_text(tl!("For the opposite half: places mirrored left to right, x moves the other way"));
        if ui.button(tl!("Copy Rules To")).clicked() {
            run(app, "grade.copyRules", json!({ "from": id, "to": [st.0], "mirror": if st.1 { "x" } else { "none" } }));
        }
    });
    set_pstate(ui.ctx(), "grading-copy", st);
}

pub fn show(app: &mut VectorcraftApp, ui: &mut Ui) {
    let Some(c) = fetch(app, ui.ctx()) else {
        widgets::dim_label(ui, tl!("No document"));
        return;
    };
    // Without a piece selected, grade.info is refused; the size range is the document's.
    let range = match &c.info {
        Some(i) => Some(i.clone()),
        None => app.session.active().and_then(|st| st.doc.grade_sizes.as_deref().map(|r| json!({"sizes": r.sizes, "base": r.base}))),
    };
    size_range(app, ui, range.as_ref());
    widgets::divider(ui);
    match &c.info {
        Some(info) => rules(app, ui, info),
        None => {
            widgets::dim_label(ui, tl!("Select one pattern piece to grade its points"));
        }
    }
    widgets::divider(ui);
    ui.horizontal(|ui| {
        if ui.button(tl!("Grade Nest")).clicked() {
            match app.run("grade.nest", json!({})) {
                // A size the pattern can't make is worth saying.
                Ok(v) => {
                    if let Some(w) = v["warnings"].as_array().and_then(|w| w.first()).and_then(Value::as_str) {
                        app.status(w.to_string());
                    }
                }
                Err(e) => app.status(e),
            }
        }
        if ui.button(tl!("Clear Graded Nest")).clicked() {
            run(app, "grade.clear", json!({}));
        }
    });
}
