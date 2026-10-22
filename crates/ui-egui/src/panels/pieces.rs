//! Pattern Pieces panel (PF-1): the document's pattern pieces, and the selected piece's name, cut
//! count, fabric, seam allowance and the edge cut on the fold. Everything it shows comes from `piece.list` and `piece.info`;
//! every change goes through `piece.set`, `piece.make` and `piece.release`.

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
}

fn fetch(app: &mut VectorcraftApp, ctx: &egui::Context) -> Option<Cache> {
    let st = app.session.active()?;
    let key = Some((st.uid, st.revision));
    let mut c: Cache = pstate(ctx, "pieces-cache");
    if c.key != key {
        c.list = app.session.execute("piece.list", &json!({})).ok().and_then(|v| v["pieces"].as_array().cloned()).unwrap_or_default();
        c.info = app.session.execute("piece.info", &json!({})).ok();
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
        let notches = info["notches"].as_array().map_or(0, Vec::len);
        super::doc_info::row(ui, tl!("Edges"), edges.to_string());
        super::doc_info::row(ui, tl!("Notches"), notches.to_string());
        super::doc_info::row(ui, tl!("Seam line length"), format!("{} mm", info["seamLength"].as_f64().unwrap_or_default()));
        if let Some(problem) = info["problem"].as_str() {
            widgets::dim_label(ui, &crate::i18n::message(crate::i18n::current(), problem));
        }
        if ui.button(tl!("Release Pattern Piece")).clicked() {
            run(app, "piece.release", json!({ "id": id }));
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
