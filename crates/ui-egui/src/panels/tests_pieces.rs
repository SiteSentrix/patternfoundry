//! The Pieces panel (PF-1), drawn headless: the selected piece's edges, each with its seam
//! allowance and a button to add a notch, and its notches, each with a button to remove it.

use serde_json::json;
use vectorcraft_engine::Session;

use crate::VectorcraftApp;

/// A 10 cm square made a pattern piece (selected).
fn square() -> (VectorcraftApp, u64) {
    let mut app = VectorcraftApp::new(Session::new(), Default::default());
    app.run("file.new", json!({"width": 800, "height": 800})).unwrap();
    app.run("shape.rectangle", json!({"x": 40, "y": 40, "width": 283.465, "height": 283.465})).unwrap();
    let id = app.run("piece.make", json!({"allowance": 10})).unwrap()["ids"][0].as_u64().unwrap();
    (app, id)
}

struct Panel {
    ctx: egui::Context,
    time: f64,
}

impl Panel {
    fn new() -> Self {
        let ctx = egui::Context::default();
        crate::theme::install_fonts(&ctx);
        Self { ctx, time: 1.0 }
    }

    /// One frame with `events`: every text drawn, with its rect.
    fn frame(&mut self, app: &mut VectorcraftApp, events: Vec<egui::Event>) -> Vec<(String, egui::Rect)> {
        self.time += 0.5;
        let raw = egui::RawInput { events, time: Some(self.time), ..Default::default() };
        super::tests_appearance::frame_raw(&self.ctx, app, raw, super::pieces::show)
    }

    fn texts(&mut self, app: &mut VectorcraftApp) -> Vec<String> {
        self.frame(app, vec![]).into_iter().map(|(t, _)| t).collect()
    }

    /// Click the first text `label`.
    fn click(&mut self, app: &mut VectorcraftApp, label: &str) {
        let at = super::tests_appearance::text_rect(&self.frame(app, vec![]), label).center();
        self.click_at(app, at);
    }

    /// Click at `at`.
    fn click_at(&mut self, app: &mut VectorcraftApp, at: egui::Pos2) {
        let button = |pressed| egui::Event::PointerButton { pos: at, button: egui::PointerButton::Primary, pressed, modifiers: Default::default() };
        for e in [egui::Event::PointerMoved(at), button(true), button(false)] {
            self.frame(app, vec![e]);
        }
        self.frame(app, vec![]);
    }
}

#[test]
fn edges_take_their_own_allowance_and_notches_are_added_and_removed() {
    let (mut app, id) = square();
    let mut panel = Panel::new();
    // Folded away at first, it says how many edges and notches there are.
    let texts = panel.texts(&mut app);
    assert!(texts.iter().any(|t| t == "Edges: 4, notches: 0") && !texts.iter().any(|t| t == "Edge 0"), "{texts:?}");
    panel.click(&mut app, "Edges: 4, notches: 0");
    let texts = panel.texts(&mut app);
    for e in 0..4 {
        assert!(texts.iter().any(|t| *t == format!("Edge {e}")), "{texts:?}");
    }
    assert_eq!(texts.iter().filter(|t| *t == "Add Notch").count(), 4);
    // A notch in the middle of the first edge, then removed.
    panel.click(&mut app, "Add Notch");
    let info = app.run("piece.info", json!({"id": id})).unwrap();
    assert_eq!(info["notches"], json!([{"edge": 0, "at": 0.5, "kind": "slit"}]));
    assert!(panel.texts(&mut app).iter().any(|t| t == "Notch on edge 0, 50% along"));
    // Its shape: a slit, picked from the list as a castle notch.
    panel.click(&mut app, "Slit notch");
    panel.click(&mut app, "Castle notch");
    assert_eq!(app.run("piece.info", json!({"id": id})).unwrap()["notches"][0]["kind"], "castle");
    panel.click(&mut app, "×");
    assert_eq!(app.run("piece.info", json!({"id": id})).unwrap().get("notches"), None);
    // An edge with its own allowance has a button back to the piece's.
    app.run("piece.set", json!({"id": id, "edges": {"2": 40}})).unwrap();
    let texts = panel.texts(&mut app);
    assert!(texts.iter().any(|t| t == "40.0") && texts.iter().filter(|t| *t == "10.0").count() == 4, "{texts:?}");
    panel.click(&mut app, "↺");
    assert_eq!(app.run("piece.info", json!({"id": id})).unwrap().get("edges"), None);
    // The fold takes no allowance.
    app.run("piece.set", json!({"id": id, "fold": 3})).unwrap();
    assert!(panel.texts(&mut app).iter().any(|t| t == "On fold"));
}

#[test]
fn drill_holes_are_listed_and_removed() {
    let (mut app, id) = square();
    let mut panel = Panel::new();
    assert!(panel.texts(&mut app).iter().any(|t| t == "Drill holes: 0"));
    app.run("piece.addDrill", json!({"id": id, "point": [100, 100], "diameter": 3})).unwrap();
    panel.click(&mut app, "Drill holes: 1");
    assert!(panel.texts(&mut app).iter().any(|t| t == "Drill hole 1, 3 mm"));
    panel.click(&mut app, "×");
    assert_eq!(app.run("piece.info", json!({"id": id})).unwrap().get("drills"), None);
    assert!(panel.texts(&mut app).iter().any(|t| t == "Right-click inside the piece: Add Drill Hole Here"));
}

#[test]
fn a_block_parts_design_steps_are_listed_and_removed() {
    let mut app = VectorcraftApp::new(Session::new(), Default::default());
    app.run("file.new", json!({"width": 1400, "height": 1000})).unwrap();
    let m =
        json!({"bust": 880, "waist": 680, "nape_to_waist": 410, "back_width": 345, "chest": 325, "shoulder": 122, "neck": 370, "armscye_depth": 210});
    app.run("measure.set", json!({"values": m})).unwrap();
    let id = app.run("block.bodice", json!({"origin": [0, 0]})).unwrap()["ids"][1].as_u64().unwrap();
    app.run("select.set", json!({"ids": [id]})).unwrap();
    let mut panel = Panel::new();
    assert!(panel.texts(&mut app).iter().any(|t| t == "Design steps: 0"));
    let r = app.run("design.rotateDart", json!({"id": id, "apex": 11, "toAnchor": 8})).unwrap();
    app.run("design.shortenDart", json!({"id": id, "apex": r["apex"], "shorten": 20})).unwrap();
    panel.click(&mut app, "Design steps: 2");
    let texts = panel.texts(&mut app);
    assert!(texts.iter().any(|t| t == "1. Dart moved") && texts.iter().any(|t| t == "2. Dart shortened 20 mm"), "{texts:?}");
    // The last step's button: the dart stays moved.
    let last = panel.frame(&mut app, vec![]).into_iter().rfind(|(t, _)| t == "×").unwrap().1;
    panel.click_at(&mut app, last.center());
    let texts = panel.texts(&mut app);
    assert!(texts.iter().any(|t| t == "1. Dart moved") && !texts.iter().any(|t| t.contains("Dart shortened")), "{texts:?}");
    assert_eq!(app.run("block.options", json!({"id": id})).unwrap()["steps"].as_array().unwrap().len(), 1);
}
