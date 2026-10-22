//! Projector view (PF-7.5, Pattern › Projector View, `ui.projector`): the pattern pieces drawn
//! full screen at true size for a projector over the cutting table, white lines on black (easier
//! to see on fabric, less glare), as projector sewists use their projector files
//! (https://www.craftstorming.com/2020/05/tips-for-using-pdf-sewing-patterns-on-a-projector),
//! without making one.
//!
//! True size on the mat needs the projector calibrated once: the view shows a 10 cm square, the
//! user measures it on the mat and makes it bigger or smaller until it measures 10 cm, and the
//! scale (screen points per millimetre on the mat) is kept in Preferences › Devices › Projector
//! Scale. The pieces come from `piece.outlines` (pieces cut on the fold opened out unless asked
//! otherwise), with a 10 cm test square beside them to check the scale by. Keys: the arrows move
//! the pattern 1 cm (with Shift 1 mm), M mirrors it (to cut with the fabric's wrong side up), C
//! calibrates again, Escape leaves.

use egui::{Color32, Key, Pos2, Shape, Stroke, Vec2};
use serde_json::{Value, json};

use crate::VectorcraftApp;

/// The test square's and the calibration square's side, mm.
const SQUARE_MM: f64 = 100.0;
/// Room left round the pieces at the start, mm.
const MARGIN_MM: f64 = 20.0;
/// The scale before calibration: a CSS inch (96 points) to 25.4 mm, what most screens show.
const GUESS: f64 = 96.0 / 25.4;

/// The projector view's state.
pub struct ProjectorState {
    pub on: bool,
    /// Showing the calibration square instead of the pattern.
    pub calibrating: bool,
    /// The page mirrored left to right.
    pub mirror: bool,
    /// How far the pattern is moved, mm.
    pub pan: [f64; 2],
    /// Pieces cut on the fold opened out.
    pub open_folds: bool,
    /// One size of the size range or chart (`None`: as drawn).
    pub size: Option<String>,
    /// The scale tried while calibrating, points per mm.
    pub trial: f64,
    /// Full screen asked of the window: `Some(on)` until sent.
    pub fullscreen: Option<bool>,
    /// The outlines, for (document uid, revision, open folds, size).
    cache: Option<((u64, u64, bool, Option<String>), Value)>,
}

impl Default for ProjectorState {
    fn default() -> Self {
        Self { on: false, calibrating: false, mirror: false, pan: [0.0, 0.0], open_folds: true, size: None, trial: GUESS, fullscreen: None, cache: None }
    }
}

/// The calibrated scale, points per mm on the mat, if there is one.
fn calibrated(app: &VectorcraftApp) -> Option<f64> {
    let s = app.session.prefs.projector_scale;
    (s.is_finite() && s > 0.0).then_some(s)
}

/// `ui.projector`: turn the view on or off (`on`; without it, the other way), calibrate
/// (`calibrate`), mirror, open the folds or not, show one size, or set the scale outright.
pub fn command(app: &mut VectorcraftApp, p: &Value) -> Result<Value, String> {
    if let Some(v) = p.get("scale") {
        let s = v.as_f64().filter(|s| s.is_finite() && *s > 0.0 && *s <= 1000.0).ok_or("scale: screen points per mm, more than 0")?;
        app.run("prefs.set", json!({"key": "projectorScale", "value": s}))?;
    }
    let on = p.get("on").and_then(Value::as_bool).unwrap_or(!app.projector.on || p.get("calibrate").is_some());
    if !on {
        if app.projector.on {
            app.projector.on = false;
            app.projector.fullscreen = Some(false);
        }
        return Ok(state(app));
    }
    let pr = &mut app.projector;
    if let Some(m) = p.get("mirror").and_then(Value::as_bool) {
        pr.mirror = m;
    }
    if let Some(o) = p.get("openFolds").and_then(Value::as_bool) {
        pr.open_folds = o;
    }
    if let Some(s) = p.get("size") {
        pr.size = s.as_str().map(str::trim).filter(|s| !s.is_empty()).map(String::from);
    }
    // The pieces must be there to show.
    outlines(app)?;
    let pr = &mut app.projector;
    if !pr.on {
        pr.on = true;
        pr.pan = [0.0, 0.0];
        pr.fullscreen = Some(true);
    }
    let scale = calibrated(app);
    let pr = &mut app.projector;
    pr.calibrating = p.get("calibrate").and_then(Value::as_bool).unwrap_or(scale.is_none());
    pr.trial = scale.unwrap_or(GUESS);
    Ok(state(app))
}

fn state(app: &VectorcraftApp) -> Value {
    let pr = &app.projector;
    json!({"on": pr.on, "calibrating": pr.calibrating, "mirror": pr.mirror, "openFolds": pr.open_folds, "scale": calibrated(app), "pan": pr.pan})
}

/// The outlines to draw, fetched again when the document changes.
fn outlines(app: &mut VectorcraftApp) -> Result<Value, String> {
    let st = app.session.active().ok_or("no document open")?;
    let key = (st.uid, st.revision, app.projector.open_folds, app.projector.size.clone());
    if let Some((k, v)) = &app.projector.cache
        && *k == key
    {
        return Ok(v.clone());
    }
    let mut p = json!({"openFolds": key.2});
    if let Some(s) = &key.3 {
        p["size"] = json!(s);
    }
    let v = app.session.execute("piece.outlines", &p).map_err(|e| e.to_string())?;
    if v["pieces"].as_array().is_none_or(Vec::is_empty) {
        return Err("no pattern pieces to project: make the outlines pattern pieces first (Pattern › Make Pattern Piece)".into());
    }
    app.projector.cache = Some((key, v.clone()));
    Ok(v)
}

/// The view's keys, taken before the app's shortcuts see them (the arrows would move the
/// selection). Returns whether the view is on, so the shortcuts are left alone while it is.
pub fn input(app: &mut VectorcraftApp, ctx: &egui::Context) -> bool {
    if let Some(f) = app.projector.fullscreen.take() {
        ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(f));
    }
    if !app.projector.on {
        return false;
    }
    let pressed = |ctx: &egui::Context, k: Key| ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, k));
    let shift = |ctx: &egui::Context, k: Key| ctx.input_mut(|i| i.consume_key(egui::Modifiers::SHIFT, k));
    if pressed(ctx, Key::Escape) {
        if app.projector.calibrating && calibrated(app).is_some() {
            app.projector.calibrating = false;
        } else {
            let _ = command(app, &json!({"on": false}));
        }
        return true;
    }
    if app.projector.calibrating {
        // Bigger or smaller by half a percent (Shift: 5%), Enter keeps it.
        for (k, f) in [(Key::ArrowUp, 1.005f64), (Key::ArrowRight, 1.005), (Key::ArrowDown, 1.0 / 1.005), (Key::ArrowLeft, 1.0 / 1.005)] {
            // Shift first: a key taken without modifiers matches with Shift held too.
            if shift(ctx, k) {
                app.projector.trial *= f.powi(10);
            }
            if pressed(ctx, k) {
                app.projector.trial *= f;
            }
        }
        if pressed(ctx, Key::Enter) {
            let s = app.projector.trial;
            if let Err(e) = app.run("prefs.set", json!({"key": "projectorScale", "value": s})) {
                app.status(e);
            }
            app.projector.calibrating = false;
        }
    } else {
        for (k, d) in [(Key::ArrowLeft, [-1.0, 0.0]), (Key::ArrowRight, [1.0, 0.0]), (Key::ArrowUp, [0.0, -1.0]), (Key::ArrowDown, [0.0, 1.0])] {
            let flip = if app.projector.mirror { -1.0 } else { 1.0 };
            if shift(ctx, k) {
                app.projector.pan[0] += d[0] * flip;
                app.projector.pan[1] += d[1];
            }
            if pressed(ctx, k) {
                app.projector.pan[0] += 10.0 * d[0] * flip;
                app.projector.pan[1] += 10.0 * d[1];
            }
        }
        if pressed(ctx, Key::M) {
            app.projector.mirror = !app.projector.mirror;
        }
        if pressed(ctx, Key::C) {
            app.projector.calibrating = true;
            app.projector.trial = calibrated(app).unwrap_or(GUESS);
        }
    }
    true
}

/// Draw the view over everything, when it is on.
pub fn frame(app: &mut VectorcraftApp, ctx: &egui::Context) {
    if !app.projector.on {
        return;
    }
    let screen = ctx.content_rect();
    let outlines = if app.projector.calibrating {
        Value::Null
    } else {
        match outlines(app) {
            Ok(v) => v,
            Err(e) => {
                app.status(e);
                let _ = command(app, &json!({"on": false}));
                return;
            }
        }
    };
    let pr = &app.projector;
    let scale = if pr.calibrating { pr.trial } else { calibrated(app).unwrap_or(GUESS) };
    let white = Color32::WHITE;
    let grey = Color32::from_gray(170);
    egui::Area::new(egui::Id::new("projector-view")).order(egui::Order::Foreground).fixed_pos(screen.min).show(ctx, |ui| {
        let (rect, _) = ui.allocate_exact_size(screen.size(), egui::Sense::click_and_drag());
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 0.0, Color32::BLACK);
        let font = |size: f32| egui::FontId::proportional(size);
        if pr.calibrating {
            // The square, in the middle of the screen.
            let side = (SQUARE_MM * scale) as f32;
            let sq = egui::Rect::from_center_size(rect.center(), Vec2::splat(side));
            painter.rect_stroke(sq, 0.0, Stroke::new(2.0, white), egui::StrokeKind::Middle);
            painter.text(sq.center_top() + Vec2::new(0.0, -12.0), egui::Align2::CENTER_BOTTOM, "10 cm", font(16.0), white);
            let text = crate::i18n::fmt(
                tl!("Measure the square on the cutting mat: it should be 10 cm each way. The arrows make it bigger or smaller (with Shift, faster); Enter keeps it; Escape leaves. Scale: {scale} points per mm."),
                &[("scale", &format!("{:.3}", scale))],
            );
            let galley = painter.layout(text, font(15.0), grey, (rect.width() - 80.0).max(200.0));
            painter.galley(Pos2::new(rect.left() + 40.0, sq.bottom() + 30.0), galley, grey);
            return;
        }
        let Some(b) = outlines["bounds"].as_array().map(|b| b.iter().filter_map(Value::as_f64).collect::<Vec<_>>()) else { return };
        let (bx, by) = (b.first().copied().unwrap_or_default(), b.get(1).copied().unwrap_or_default());
        // A document point in mm to the screen: the pieces' top-left corner past the test square's
        // band and the margin, then moved; mirrored left to right when asked.
        let origin = [bx - MARGIN_MM, by - MARGIN_MM - SQUARE_MM - MARGIN_MM];
        let to = |q: &Value| -> Option<Pos2> {
            let (x, y) = (q.get(0)?.as_f64()?, q.get(1)?.as_f64()?);
            let sx = rect.left() + ((x - origin[0] + pr.pan[0]) * scale) as f32;
            let sy = rect.top() + ((y - origin[1] + pr.pan[1]) * scale) as f32;
            Some(Pos2::new(if pr.mirror { rect.right() - (sx - rect.left()) } else { sx }, sy))
        };
        let pts = |v: &Value| -> Vec<Pos2> { v.as_array().map(|a| a.iter().filter_map(to).collect()).unwrap_or_default() };
        // The test square, above the pieces' left end.
        let corner = to(&json!([bx, by - SQUARE_MM - MARGIN_MM]));
        if let Some(c) = corner {
            let side = (SQUARE_MM * scale) as f32;
            let sq = if pr.mirror { egui::Rect::from_min_size(Pos2::new(c.x - side, c.y), Vec2::splat(side)) } else { egui::Rect::from_min_size(c, Vec2::splat(side)) };
            painter.rect_stroke(sq, 0.0, Stroke::new(2.0, white), egui::StrokeKind::Middle);
            painter.text(sq.left_top() + Vec2::new(8.0, 8.0), egui::Align2::LEFT_TOP, "10 cm", font(14.0), white);
        }
        for piece in outlines["pieces"].as_array().into_iter().flatten() {
            let cut = pts(&piece["cut"]);
            if cut.len() >= 3 {
                painter.add(Shape::closed_line(cut, Stroke::new(3.0, white)));
            }
            let mut seam = pts(&piece["seam"]);
            if seam.len() >= 3 {
                seam.push(seam[0]);
                painter.extend(Shape::dashed_line(&seam, Stroke::new(1.0, grey), 8.0, 5.0));
            }
            for n in piece["notches"].as_array().into_iter().flatten() {
                painter.add(Shape::line(pts(n), Stroke::new(3.0, white)));
            }
            for l in piece["lines"].as_array().into_iter().flatten() {
                painter.add(Shape::line(pts(l), Stroke::new(1.0, white)));
            }
            if let [a, b] = pts(&piece["grain"])[..] {
                painter.arrow(a, b - a, Stroke::new(1.5, white));
            }
            let fold = pts(&piece["fold"]);
            if fold.len() == 2 {
                painter.extend(Shape::dashed_line(&fold, Stroke::new(3.0, white), 14.0, 7.0));
            }
            if let Some(at) = to(&piece["label"]) {
                painter.text(at + Vec2::new(8.0, 0.0), egui::Align2::LEFT_CENTER, piece["text"].as_str().unwrap_or_default(), font(18.0), white);
            }
        }
        let help = tl!("Arrows move the pattern 1 cm (Shift: 1 mm) · M mirrors it · C calibrates · Escape leaves");
        painter.text(rect.center_bottom() + Vec2::new(0.0, -12.0), egui::Align2::CENTER_BOTTOM, help, font(13.0), Color32::from_gray(110));
    });
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use vectorcraft_engine::Session;

    use crate::VectorcraftApp;

    fn key(k: egui::Key, shift: bool) -> egui::Event {
        let modifiers = if shift { egui::Modifiers::SHIFT } else { egui::Modifiers::NONE };
        egui::Event::Key { key: k, physical_key: None, pressed: true, repeat: false, modifiers }
    }

    /// One frame of the app's projector handling with `events`, then a frame with their keys let
    /// go (a key held down repeats).
    fn frame(app: &mut VectorcraftApp, ctx: &egui::Context, events: Vec<egui::Event>) {
        let ups: Vec<egui::Event> = events
            .iter()
            .filter_map(|e| match e {
                egui::Event::Key { key, modifiers, .. } => {
                    Some(egui::Event::Key { key: *key, physical_key: None, pressed: false, repeat: false, modifiers: *modifiers })
                }
                _ => None,
            })
            .collect();
        one(app, ctx, events);
        if !ups.is_empty() {
            one(app, ctx, ups);
        }
    }

    fn one(app: &mut VectorcraftApp, ctx: &egui::Context, events: Vec<egui::Event>) {
        let raw = egui::RawInput { events, screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1600.0, 1000.0))), ..Default::default() };
        let mut out = ctx.run_ui(raw, |ui| {
            let ctx = ui.ctx().clone();
            super::input(app, &ctx);
            super::frame(app, &ctx);
        });
        out.textures_delta.clear();
    }

    #[test]
    fn the_view_calibrates_once_then_shows_the_pieces_and_takes_its_keys() {
        let mut app = VectorcraftApp::new(Session::new(), Default::default());
        app.run("file.new", json!({"width": 800, "height": 800})).unwrap();
        // Without pieces there is nothing to project.
        assert!(app.run("ui.projector", json!({})).is_err());
        app.run("shape.rectangle", json!({"x": 40, "y": 40, "width": 283.465, "height": 283.465})).unwrap();
        let id = app.run("piece.make", json!({"allowance": 10})).unwrap()["ids"][0].as_u64().unwrap();
        app.run("piece.set", json!({"id": id, "fold": 3})).unwrap();
        // Not calibrated: it starts on the calibration square, full screen.
        let r = app.run("ui.projector", json!({})).unwrap();
        assert_eq!((r["on"].as_bool(), r["calibrating"].as_bool(), r["scale"].clone()), (Some(true), Some(true), json!(null)));
        assert_eq!(app.projector.fullscreen, Some(true));
        let ctx = egui::Context::default();
        crate::theme::install_fonts(&ctx);
        frame(&mut app, &ctx, vec![]);
        assert_eq!(app.projector.fullscreen, None, "full screen asked of the window");
        // Two steps bigger, one ten times smaller, then kept.
        let guess = app.projector.trial;
        frame(&mut app, &ctx, vec![key(egui::Key::ArrowUp, false)]);
        frame(&mut app, &ctx, vec![key(egui::Key::ArrowUp, false)]);
        frame(&mut app, &ctx, vec![key(egui::Key::ArrowDown, true)]);
        frame(&mut app, &ctx, vec![key(egui::Key::Enter, false)]);
        let kept = app.session.prefs.projector_scale;
        assert!((kept - guess * 1.005f64.powi(-8)).abs() < 1e-9, "{kept} {guess}");
        assert!(!app.projector.calibrating);
        // The pattern: the arrows move it, not the selection; M mirrors it.
        app.run("select.set", json!({"ids": [id]})).unwrap();
        let before = app.run("piece.info", json!({"id": id})).unwrap();
        frame(&mut app, &ctx, vec![key(egui::Key::ArrowRight, false), key(egui::Key::ArrowDown, true)]);
        assert_eq!(app.projector.pan, [10.0, 1.0]);
        frame(&mut app, &ctx, vec![key(egui::Key::M, false)]);
        assert!(app.projector.mirror);
        assert_eq!(app.run("piece.info", json!({"id": id})).unwrap(), before, "the piece did not move");
        // The fold opened out by default: the outlines the view draws are the whole piece.
        let o = app.session.execute("piece.outlines", &json!({})).unwrap();
        let b: Vec<f64> = o["bounds"].as_array().unwrap().iter().map(|v| v.as_f64().unwrap()).collect();
        assert!((b[2] - b[0] - 220.0).abs() < 0.01, "{b:?}");
        // Escape leaves, and the window leaves full screen.
        one(&mut app, &ctx, vec![key(egui::Key::Escape, false)]);
        assert!(!app.projector.on);
        assert_eq!(app.projector.fullscreen, Some(false));
        one(&mut app, &ctx, vec![]);
        assert_eq!(app.projector.fullscreen, None, "full screen left");
        // Calibrated, it opens on the pattern.
        let r = app.run("ui.projector", json!({})).unwrap();
        assert_eq!((r["calibrating"].as_bool(), r["scale"].as_f64()), (Some(false), Some(kept)));
        app.run("ui.projector", json!({"on": false})).unwrap();
    }
}
