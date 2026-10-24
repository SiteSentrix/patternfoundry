//! Pattern Pieces panel (PF-1): the document's pattern pieces, and the selected piece's name, cut
//! count, fabric, seam allowance, the edge cut on the fold, and its edges (each one's allowance)
//! and notches, and its drill holes. Everything it shows comes from `piece.list` and `piece.info`;
//! every change goes through `piece.set`, `piece.addNotch`, `piece.setNotch` (a notch's shape),
//! `piece.removeNotch`, `piece.removeDrill`, `piece.make` and `piece.release`. For a drafted block
//! part (or a piece made from one) it lists the design steps made on it (`block.options`), each with
//! a button to remove it and the steps after it (`block.removeStep`, PF-5.22).

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
    list: Vec<Value>,
    info: Option<Value>,
    /// `block.options` of the selected block part.
    block: Option<Value>,
    /// `piece.seams`: every pair of runs sewn together.
    seams: Vec<Value>,
}

fn fetch(app: &mut VectorcraftApp, ctx: &egui::Context) -> Option<Cache> {
    let st = app.session.active()?;
    let key = Some((st.uid, st.revision));
    let mut c: Cache = pstate(ctx, "pieces-cache");
    if c.key != key {
        c.list = app.session.execute("piece.list", &json!({})).ok().and_then(|v| v["pieces"].as_array().cloned()).unwrap_or_default();
        c.info = app.session.execute("piece.info", &json!({})).ok();
        c.block = app.session.execute("block.options", &json!({})).ok();
        c.seams = app.session.execute("piece.seams", &json!({})).ok().and_then(|v| v["seams"].as_array().cloned()).unwrap_or_default();
        c.key = key;
        set_pstate(ctx, "pieces-cache", c.clone());
    }
    Some(c)
}

/// Run `id` with `p`; a refusal goes to the status bar.
fn run(app: &mut VectorcraftApp, id: &str, p: Value) {
    if let Err(e) = app.run(id, p) {
        app.status(e);
    }
}

/// A one-line text field that changes the piece when it loses focus (or Enter is pressed), so
/// typing a name is one undo step.
fn text_field(app: &mut VectorcraftApp, ui: &mut Ui, label: &str, key: &str, piece: u64, current: &str) {
    let id = format!("pieces-edit-{key}");
    let mut buf: (u64, String) = pstate(ui.ctx(), &id);
    if buf.0 != piece {
        buf = (piece, current.to_string());
    }
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(label).size(12.0).color(Tokens::get(ui.ctx()).text_dim));
        let r = ui.add(egui::TextEdit::singleline(&mut buf.1).desired_width(f32::INFINITY));
        if r.lost_focus() && buf.1 != current {
            run(app, "piece.set", json!({ "id": piece, key: buf.1 }));
        }
        if !r.has_focus() && !r.lost_focus() {
            buf.1 = current.to_string();
        }
    });
    set_pstate(ui.ctx(), &id, buf);
}

/// The piece's edges, each with its seam allowance (the piece's own until one is set; none on
/// the fold) and a button to add a notch at its middle, then the notches, each with a button to
/// remove it.
fn edges_section(app: &mut VectorcraftApp, ui: &mut Ui, info: &Value, id: u64, edges: u64) {
    let t = Tokens::get(ui.ctx());
    let default = info["allowance"].as_f64().unwrap_or(10.0);
    let notches = info["notches"].as_array().cloned().unwrap_or_default();
    let title =
        crate::i18n::fmt(tl!("Edges: {edges}, notches: {notches}"), &[("edges", &edges.to_string()), ("notches", &notches.len().to_string())]);
    egui::CollapsingHeader::new(title).id_salt("pieces-edges").show(ui, |ui| {
        egui::Grid::new("pieces-edge-grid").num_columns(3).spacing([8.0, 4.0]).show(ui, |ui| {
            for e in 0..edges {
                ui.label(egui::RichText::new(crate::i18n::fmt(tl!("Edge {n}"), &[("n", &e.to_string())])).size(12.0).color(t.text_dim));
                ui.horizontal(|ui| {
                    if info["fold"].as_u64() == Some(e) {
                        ui.label(egui::RichText::new(tl!("On fold")).size(12.0));
                        return;
                    }
                    let own = info["edges"].get(e.to_string()).and_then(Value::as_f64);
                    let mut mm = own.unwrap_or(default);
                    let r = ui.add(egui::DragValue::new(&mut mm).range(0.0..=200.0).speed(0.5).max_decimals(2).suffix(" mm"));
                    if r.drag_stopped() || (r.changed() && !r.dragged()) {
                        run(app, "piece.set", json!({ "id": id, "edges": { e.to_string(): mm } }));
                    }
                    if own.is_some() && ui.small_button("↺").on_hover_text(tl!("Back to the piece's seam allowance")).clicked() {
                        run(app, "piece.set", json!({ "id": id, "edges": { e.to_string(): Value::Null } }));
                    }
                });
                if ui.small_button(tl!("Add Notch")).on_hover_text(tl!("A notch in the middle of the edge")).clicked() {
                    run(app, "piece.addNotch", json!({ "id": id, "edge": e, "at": 0.5 }));
                }
                ui.end_row();
            }
        });
        for (i, n) in notches.iter().enumerate() {
            ui.horizontal(|ui| {
                let at = (n["at"].as_f64().unwrap_or_default() * 100.0).round().to_string();
                let edge = n["edge"].as_u64().unwrap_or_default().to_string();
                ui.label(egui::RichText::new(crate::i18n::fmt(tl!("Notch on edge {edge}, {at}% along"), &[("edge", &edge), ("at", &at)])).size(12.0));
                let kind = n["kind"].as_str().unwrap_or("slit");
                let name = |k: &str| match k {
                    "double" => tl!("Double notch"),
                    "v" => tl!("V notch"),
                    "t" => tl!("T notch"),
                    "castle" => tl!("Castle notch"),
                    "u" => tl!("U notch"),
                    _ => tl!("Slit notch"),
                };
                egui::ComboBox::from_id_salt(("pieces-notch-kind", i)).selected_text(name(kind)).show_ui(ui, |ui| {
                    for k in ["slit", "double", "v", "t", "castle", "u"] {
                        if ui.selectable_label(k == kind, name(k)).clicked() && k != kind {
                            run(app, "piece.setNotch", json!({ "id": id, "index": i, "kind": k }));
                        }
                    }
                });
                if ui.small_button("×").on_hover_text(tl!("Remove Notch")).clicked() {
                    run(app, "piece.removeNotch", json!({ "id": id, "index": i }));
                }
            });
        }
    });
}

/// The piece's drill holes, each with its punch's diameter and a button to remove it; they are
/// added by right-clicking inside the piece.
fn drills_section(app: &mut VectorcraftApp, ui: &mut Ui, info: &Value, id: u64) {
    let drills = info["drills"].as_array().cloned().unwrap_or_default();
    let title = crate::i18n::fmt(tl!("Drill holes: {count}"), &[("count", &drills.len().to_string())]);
    egui::CollapsingHeader::new(title).id_salt("pieces-drills").show(ui, |ui| {
        if drills.is_empty() {
            widgets::dim_label(ui, tl!("Right-click inside the piece: Add Drill Hole Here"));
        }
        for (i, d) in drills.iter().enumerate() {
            ui.horizontal(|ui| {
                let n = (i + 1).to_string();
                let text = match d["diameter"].as_f64().filter(|v| *v > 0.0) {
                    Some(mm) => crate::i18n::fmt(tl!("Drill hole {n}, {mm} mm"), &[("n", &n), ("mm", &mm.to_string())]),
                    None => crate::i18n::fmt(tl!("Drill hole {n}"), &[("n", &n)]),
                };
                ui.label(egui::RichText::new(text).size(12.0));
                if ui.small_button("×").on_hover_text(tl!("Remove Drill Hole")).clicked() {
                    run(app, "piece.removeDrill", json!({ "id": id, "index": i }));
                }
            });
        }
    });
}

/// The seams the piece `id` is sewn by (PF-10.2): each run's label, the piece it is sewn to and
/// how much longer or shorter it is, with a button to forget it.
fn sewn_section(app: &mut VectorcraftApp, ui: &mut Ui, seams: &[Value], id: u64) {
    let mine: Vec<&Value> =
        seams.iter().filter(|sm| sm["runs"].as_array().is_some_and(|r| r.iter().any(|r| r["piece"].as_u64() == Some(id)))).collect();
    if mine.is_empty() {
        return;
    }
    let title = crate::i18n::fmt(tl!("Sewn seams: {count}"), &[("count", &mine.len().to_string())]);
    egui::CollapsingHeader::new(title).id_salt("pieces-sewn").show(ui, |ui| {
        for sm in mine {
            let label = sm["label"].as_str().unwrap_or_default();
            let runs = sm["runs"].as_array().cloned().unwrap_or_default();
            let other = runs.iter().find(|r| r["piece"].as_u64() != Some(id)).or(runs.first());
            let name = other.and_then(|r| r["name"].as_str()).filter(|n| !n.is_empty()).unwrap_or(tl!("Pattern piece"));
            // This piece's run against the other's.
            let diff = sm["difference"].as_f64().map(|d| if runs.first().and_then(|r| r["piece"].as_u64()) == Some(id) { d } else { -d });
            ui.horizontal(|ui| {
                let text = match diff {
                    Some(d) if d.abs() >= 0.5 => crate::i18n::fmt(
                        if d > 0.0 { tl!("{label}: sewn to {piece}, {mm} mm longer") } else { tl!("{label}: sewn to {piece}, {mm} mm shorter") },
                        &[("label", label), ("piece", name), ("mm", &((d.abs() * 10.0).round() / 10.0).to_string())],
                    ),
                    _ => crate::i18n::fmt(tl!("{label}: sewn to {piece}"), &[("label", label), ("piece", name)]),
                };
                ui.label(egui::RichText::new(text).size(12.0));
                if ui.small_button("×").on_hover_text(tl!("Unsew Seams")).clicked() {
                    run(app, "piece.unsew", json!({ "label": label }));
                }
            });
        }
    });
}

/// A design step, as `block.options` lists it, in words.
fn step_text(step: &Value) -> String {
    let mm = |k: &str| step[k].as_f64().map(|v| ((v * 10.0).round() / 10.0).to_string()).unwrap_or_default();
    match step["step"].as_str().unwrap_or_default() {
        "rotateDart" => tl!("Dart moved").to_string(),
        "shortenDart" => crate::i18n::fmt(tl!("Dart shortened {mm} mm"), &[("mm", &mm("by"))]),
        "curveDart" => crate::i18n::fmt(tl!("Dart legs curved {mm} mm"), &[("mm", &mm("depth"))]),
        "spread" => crate::i18n::fmt(tl!("Slashed and spread {mm} mm"), &[("mm", &mm("amount"))]),
        "matchLength" => crate::i18n::fmt(tl!("Seam trued to {mm} mm"), &[("mm", &mm("length"))]),
        "split" => tl!("Split into panels").to_string(),
        _ => tl!("Design step").to_string(),
    }
}

/// The design steps made on the selected block part, in order, each with a button to remove it and
/// the steps after it; the part is drafted again from the measurements and the steps before it
/// made again. A split stays (it made two panels).
fn steps_section(app: &mut VectorcraftApp, ui: &mut Ui, block: &Value) {
    let Some(id) = block["id"].as_u64() else { return };
    let steps = block["steps"].as_array().cloned().unwrap_or_default();
    let title = crate::i18n::fmt(tl!("Design steps: {count}"), &[("count", &steps.len().to_string())]);
    egui::CollapsingHeader::new(title).id_salt("pieces-steps").show(ui, |ui| {
        if steps.is_empty() {
            widgets::dim_label(ui, tl!("Design changes made on the block (darts, splits, spreads, seam lengths) show here; each is made again when the measurements change"));
        }
        // Steps up to the last split stay.
        let kept = steps.iter().rposition(|st| st["step"] == "split").map_or(0, |i| i + 1);
        for (i, step) in steps.iter().enumerate() {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(format!("{}. {}", i + 1, step_text(step))).size(12.0));
                if i >= kept && ui.small_button("×").on_hover_text(tl!("Remove this step and the ones after it (they were made on the outline it made)")).clicked() {
                    run(app, "block.removeStep", json!({ "id": id, "index": i }));
                }
            });
        }
    });
}

pub fn show(app: &mut VectorcraftApp, ui: &mut Ui) {
    let Some(c) = fetch(app, ui.ctx()) else {
        widgets::dim_label(ui, tl!("No document"));
        return;
    };
    let t = Tokens::get(ui.ctx());
    if let Some(info) = &c.info {
        let id = info["id"].as_u64().unwrap_or_default();
        widgets::subheader(ui, tl!("Selected piece"));
        text_field(app, ui, tl!("Name"), "name", id, info["name"].as_str().unwrap_or_default());
        text_field(app, ui, tl!("Fabric"), "fabric", id, info["fabric"].as_str().unwrap_or_default());
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(tl!("Cut")).size(12.0).color(t.text_dim));
            let mut cut = info["cut"].as_u64().unwrap_or(1);
            if ui.add(egui::DragValue::new(&mut cut).range(1..=999)).changed() {
                run(app, "piece.set", json!({ "id": id, "cut": cut }));
            }
            let mut mirror = info["mirror"].as_bool().unwrap_or(false);
            if ui.checkbox(&mut mirror, tl!("Mirrored pairs")).changed() {
                run(app, "piece.set", json!({ "id": id, "mirror": mirror }));
            }
        });
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(tl!("Seam allowance")).size(12.0).color(t.text_dim));
            let mut mm = info["allowance"].as_f64().unwrap_or(10.0);
            let r = ui.add(egui::DragValue::new(&mut mm).range(0.0..=200.0).speed(0.5).max_decimals(2).suffix(" mm"));
            // A drag is one change when it ends; typing, when the field is left.
            if r.drag_stopped() || (r.changed() && !r.dragged()) {
                run(app, "piece.set", json!({ "id": id, "allowance": mm }));
            }
        });
        let edges = info["edgeCount"].as_u64().unwrap_or_default();
        ui.horizontal(|ui| {
            let fold = info["fold"].as_u64();
            let mut on = fold.is_some();
            let r = ui.checkbox(&mut on, tl!("Cut on fold")).on_hover_text(tl!("The straight edge laid on the fabric's fold: no allowance there"));
            if r.changed() {
                run(app, "piece.set", json!({ "id": id, "fold": if on { json!("auto") } else { Value::Null } }));
            }
            if let Some(mut e) = fold {
                ui.label(egui::RichText::new(tl!("Edge")).size(12.0).color(t.text_dim));
                if ui.add(egui::DragValue::new(&mut e).range(0..=edges.saturating_sub(1))).changed() {
                    run(app, "piece.set", json!({ "id": id, "fold": e }));
                }
            }
        });
        edges_section(app, ui, info, id, edges);
        drills_section(app, ui, info, id);
        sewn_section(app, ui, &c.seams, id);
        if let Some(block) = &c.block {
            steps_section(app, ui, block);
        }
        super::doc_info::row(ui, tl!("Seam line length"), format!("{} mm", info["seamLength"].as_f64().unwrap_or_default()));
        if let Some(problem) = info["problem"].as_str() {
            widgets::dim_label(ui, &crate::i18n::message(crate::i18n::current(), problem));
        }
        if ui.button(tl!("Release Pattern Piece")).clicked() {
            run(app, "piece.release", json!({ "id": id }));
        }
        widgets::divider(ui);
    } else if let Some(block) = &c.block {
        widgets::subheader(ui, tl!("Selected block part"));
        steps_section(app, ui, block);
        if ui.button(tl!("Make Pattern Piece")).on_hover_text(tl!("Each selected closed path becomes a piece: the path is its seam line")).clicked() {
            run(app, "piece.make", json!({}));
        }
        widgets::divider(ui);
    } else if ui
        .button(tl!("Make Pattern Piece"))
        .on_hover_text(tl!("Each selected closed path becomes a piece: the path is its seam line"))
        .clicked()
    {
        run(app, "piece.make", json!({}));
    }
    widgets::subheader(ui, tl!("Pieces"));
    if c.list.is_empty() {
        widgets::dim_label(ui, tl!("No pattern pieces yet"));
        return;
    }
    let selected = c.info.as_ref().and_then(|i| i["id"].as_u64());
    egui::ScrollArea::vertical().max_height(260.0).show(ui, |ui| {
        for p in &c.list {
            let id = p["id"].as_u64().unwrap_or_default();
            let name = p["name"].as_str().filter(|n| !n.is_empty()).unwrap_or(tl!("Pattern piece"));
            let cut = crate::i18n::fmt(tl!("Cut {count}"), &[("count", &p["cut"].as_u64().unwrap_or(1).to_string())]);
            let text = format!("{name}  ·  {cut}");
            if ui.selectable_label(selected == Some(id), text).clicked() {
                run(app, "select.set", json!({ "ids": [id] }));
            }
        }
    });
}
