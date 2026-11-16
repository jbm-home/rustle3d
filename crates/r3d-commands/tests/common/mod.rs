//! Shared by the integration tests: the A-line skirt template.
#![allow(dead_code, clippy::unwrap_used)] // each test file uses part of it

use r3d_commands::{Session, Step};
use serde_json::{Value, json};

/// Ids (JSON strings) of a panel, its points and its contour edges, in
/// drawing order (edge k leaves point k).
pub struct Piece {
    pub panel: Value,
    pub points: Vec<Value>,
    pub edges: Vec<Value>,
}

/// The panel called `name`.
pub fn piece(s: &Session, name: &str) -> Piece {
    let p = s.project();
    let (id, panel) = p.panels().iter().find(|(_, pn)| pn.name == name).unwrap();
    Piece {
        panel: json!(id.to_string()),
        points: panel
            .contour
            .iter()
            .map(|e| json!(p.edge(*e).unwrap().start.to_string()))
            .collect(),
        edges: panel.contour.iter().map(|e| json!(e.to_string())).collect(),
    }
}

/// Draft the A-line skirt, 60 cm long, and return its front: points
/// hem left, hem right, waist right, waist left; edges hem, right side,
/// waist, left side.
pub fn a_line_skirt(s: &mut Session) -> Piece {
    let step = json!({"type": "template.apply", "template": "a_line_skirt",
        "sizes": {"length_cm": 60}});
    s.run_step(&Step::from_value(step).unwrap()).unwrap();
    piece(s, "Front")
}

/// The standard women's M body.
pub fn women_m(s: &mut Session) {
    let body = json!({"type": "avatar.set", "preset": "Women M",
        "body": {"masculinity": 0, "height_cm": 168, "bust_cm": 90, "waist_cm": 73, "hips_cm": 98}});
    s.run_step(&Step::from_value(body).unwrap()).unwrap();
}
