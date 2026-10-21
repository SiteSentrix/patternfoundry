//! The Grading panel (PF-6), drawn headless: the size range, the selected piece's rules and the
//! nest buttons.

use serde_json::json;
use vectorcraft_engine::Session;

use crate::VectorcraftApp;
use crate::tests_labels::painted_text;

fn panel(app: &mut VectorcraftApp) -> String {
    painted_text(app, super::grading::show)
}

/// A 10 cm square made a pattern piece (selected), in a document graded S, M, L.
fn square() -> (VectorcraftApp, u64) {
    let mut app = VectorcraftApp::new(Session::new(), Default::default());
    app.run("file.new", json!({"width": 800, "height": 800})).unwrap();
    app.run("shape.rectangle", json!({"x": 40, "y": 40, "width": 283.465, "height": 283.465})).unwrap();
    let id = app.run("piece.make", json!({"allowance": 10})).unwrap()["ids"][0].as_u64().unwrap();
    app.run("grade.setSizes", json!({"sizes": ["S", "M", "L"], "base": "M"})).unwrap();
    (app, id)
}

/// Click the text `label` in the panel.
fn click(app: &mut VectorcraftApp, label: &str) {
    let ctx = egui::Context::default();
    crate::theme::install_fonts(&ctx);
    let frame = |app: &mut VectorcraftApp, events: Vec<egui::Event>, time: f64| {
        super::tests_appearance::frame_raw(&ctx, app, egui::RawInput { events, time: Some(time), ..Default::default() }, super::grading::show)
    };
    let at = super::tests_appearance::text_rect(&frame(app, vec![], 1.0), label).center();
    let button = |pressed| egui::Event::PointerButton { pos: at, button: egui::PointerButton::Primary, pressed, modifiers: Default::default() };
    for (i, e) in [egui::Event::PointerMoved(at), button(true), button(false)].into_iter().enumerate() {
        frame(app, vec![e], 1.0 + 0.01 * (i + 1) as f64);
    }
}

#[test]
fn without_a_piece_the_panel_asks_for_one() {
    let mut app = VectorcraftApp::new(Session::new(), Default::default());
    app.run("file.new", json!({"width": 100, "height": 100})).unwrap();
    let text = panel(&mut app);
    assert!(text.contains("Sizes") && text.contains("Select one pattern piece to grade its points"), "{text}");
}

#[test]
fn the_selected_piece_shows_a_row_per_point_and_the_nest_buttons_work() {
    let (mut app, id) = square();
    let text = panel(&mut app);
    assert!(text.contains("Grade rules (mm a size)") && text.contains("S, M, L"), "{text}");
    let points = app.run("grade.info", json!({"id": id})).unwrap()["moves"]["M"].as_array().unwrap().len();
    for i in 0..points {
        assert!(text.lines().any(|l| l == i.to_string()), "point {i}: {text}");
    }
    // A rule naming each size shows as one.
    app.run("grade.setRule", json!({"id": id, "anchor": 1, "sizes": {"L": [5, 0]}})).unwrap();
    assert!(panel(&mut app).contains("per size"));
    click(&mut app, "Grade Nest");
    let has_nest = |app: &VectorcraftApp| app.session.active().unwrap().doc.layers.iter().any(|l| l.name.as_deref() == Some("Graded Nest"));
    assert!(has_nest(&app));
    click(&mut app, "Clear Graded Nest");
    assert!(!has_nest(&app));
}

#[test]
fn set_size_range_sends_the_typed_sizes() {
    let mut app = VectorcraftApp::new(Session::new(), Default::default());
    app.run("file.new", json!({"width": 100, "height": 100})).unwrap();
    // As if typed into the Sizes field.
    let ctx = egui::Context::default();
    super::set_pstate(&ctx, "grading-sizes", ("10, 12, 14".to_string(), String::new(), String::new()));
    let frame = |app: &mut VectorcraftApp, events: Vec<egui::Event>, time: f64| {
        super::tests_appearance::frame_raw(&ctx, app, egui::RawInput { events, time: Some(time), ..Default::default() }, super::grading::show)
    };
    crate::theme::install_fonts(&ctx);
    let at = super::tests_appearance::text_rect(&frame(&mut app, vec![], 1.0), "Set Size Range").center();
    let button = |pressed| egui::Event::PointerButton { pos: at, button: egui::PointerButton::Primary, pressed, modifiers: Default::default() };
    for (i, e) in [egui::Event::PointerMoved(at), button(true), button(false)].into_iter().enumerate() {
        frame(&mut app, vec![e], 1.0 + 0.01 * (i + 1) as f64);
    }
    let range = app.session.active().unwrap().doc.grade_sizes.clone().unwrap();
    assert_eq!(range.sizes, ["10", "12", "14"]);
    // No base chosen: the middle size.
    assert_eq!(range.base, "12");
}
