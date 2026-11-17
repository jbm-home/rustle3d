//! Every command: apply → undo → redo restores exact states, and the
//! command survives a JSON round-trip. The coverage check at the end fails
//! when a new command is registered without being exercised here.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // test code

use std::collections::BTreeSet;

use r3d_commands::step::parse_command;
use r3d_commands::{Command, CommandOutput, Session};
use r3d_core::Project;
use serde_json::{Value, json};

struct Harness {
    session: Session,
    covered: BTreeSet<&'static str>,
}

impl Harness {
    fn new() -> Self {
        Self {
            session: Session::new(Project::new("test")),
            covered: BTreeSet::new(),
        }
    }

    fn round_trip(&mut self, value: Value) -> CommandOutput {
        let cmd = parse_command(value).unwrap();
        let back: Command = serde_json::from_value(serde_json::to_value(&cmd).unwrap()).unwrap();
        assert_eq!(back, cmd, "JSON round-trip");

        let before = self.session.project().clone();
        let out = self.session.execute(&cmd).unwrap();
        let after = self.session.project().clone();
        assert_ne!(before, after, "{} changed nothing", cmd.name());
        after.check_integrity().unwrap();

        self.session.undo().unwrap();
        assert_eq!(self.session.project(), &before, "undo of {}", cmd.name());
        self.session.redo().unwrap();
        assert_eq!(self.session.project(), &after, "redo of {}", cmd.name());
        self.session.seal_history();

        self.covered.insert(cmd.name());
        out
    }
}

#[test]
fn every_command_round_trips() {
    let mut h = Harness::new();

    h.round_trip(json!({"type": "project.rename", "name": "Skirt"}));
    h.round_trip(json!({"type": "project.set_display_unit", "unit": "cm"}));
    let body =
        json!({"masculinity": 0, "height_cm": 168, "bust_cm": 90, "waist_cm": 73, "hips_cm": 98});
    h.round_trip(json!({"type": "avatar.set", "body": body, "preset": "Women M"}));
    h.round_trip(json!({"type": "avatar.remove"}));

    let rect = h.round_trip(json!({
        "type": "panel.create_rect", "name": "Front", "origin": [10, 20], "width": 400, "height": 600
    }));
    assert_eq!(rect.created.len(), 1 + 4 + 4);
    let panel = rect.created[0].to_string();
    let point = rect.created[2].to_string();
    let other_point = rect.created[1].to_string();
    let edge = rect.created[6].to_string();

    let poly = h.round_trip(json!({
        "type": "panel.create_polygon",
        // Clockwise input: stored counter-clockwise.
        "points": [[0, 0], [0, 600], [360, 600], [400, 0]]
    }));
    assert_eq!(poly.created.len(), 1 + 4 + 4);

    h.round_trip(json!({"type": "panel.rename", "panel": panel, "name": "Front skirt"}));
    h.round_trip(json!({"type": "edge.set_curve", "edge": edge,
        "curve": {"kind": "cubic", "c1": [300, 650], "c2": [120, 650]}}));
    h.round_trip(json!({"type": "point.move", "point": point, "to": [420, 0]}));
    h.round_trip(json!({"type": "point.set_distance", "from": other_point, "point": point, "distance_mm": 250}));
    h.round_trip(json!({"type": "panel.translate", "panel": panel, "delta": [100, -50]}));
    h.round_trip(json!({"type": "panel.set_grain", "panel": panel, "angle_deg": 45}));
    h.round_trip(json!({"type": "panel.set_seam_allowance", "panel": panel, "mm": 15}));
    h.round_trip(json!({"type": "panel.place", "panel": panel, "angle_deg": 0, "top_cm": 104}));
    h.round_trip(json!({"type": "garment.set_fabric", "fabric": "denim"}));
    h.round_trip(json!({"type": "garment.set_color", "color": "#335577"}));
    let jacket = h.round_trip(json!({"type": "garment.add", "name": "Jacket",
        "fabric": "wool", "color": "#223344"}));
    let jacket = jacket.created[0].to_string();
    h.round_trip(json!({"type": "garment.rename", "garment": jacket, "name": "Coat"}));
    h.round_trip(json!({"type": "panel.set_garment", "panel": panel, "garment": jacket}));
    h.round_trip(json!({"type": "garment.set_layer", "garment": jacket, "layer": 0}));
    let poly_edge = poly.created[5].to_string();
    let other_edge = rect.created[7].to_string();
    let seam = h.round_trip(json!({"type": "seam.add",
        "a": {"edge": poly_edge}, "b": {"edge": other_edge, "reversed": true}}));
    let seam = seam.created[0].to_string();
    h.round_trip(json!({"type": "seam.flip", "seam": seam}));
    h.round_trip(json!({"type": "seam.set_gathered", "seam": seam, "gathered": true}));
    h.round_trip(json!({"type": "seam.take_in", "seam": seam, "mm": 6}));
    h.round_trip(json!({"type": "seam.match_length", "seam": seam, "keep": poly_edge}));
    h.round_trip(json!({"type": "garment.arrange"}));
    h.round_trip(json!({"type": "template.apply", "template": "t_shirt",
        "sizes": {"chest_cm": 100}}));
    h.round_trip(json!({"type": "seam.remove", "seam": seam}));
    h.round_trip(json!({"type": "edge.lengthen", "edge": poly_edge, "mm": 12}));
    h.round_trip(json!({"type": "edge.set_length", "edge": poly_edge, "length_mm": 150}));
    h.round_trip(json!({"type": "edge.move", "edge": poly_edge, "by": [5, -8]}));
    let pin = h.round_trip(json!({"type": "pin.add", "panel": panel,
        "at": [10, 20], "position": [0.0, 1.0, 0.1]}));
    let pin = pin.created[0].to_string();
    h.round_trip(json!({"type": "pin.remove", "pin": pin}));

    let split = h.round_trip(json!({"type": "edge.split", "edge": edge, "at_mm": 100}));
    let new_point = split.created[0].to_string();
    let notch = h.round_trip(json!({"type": "notch.add", "edge": edge, "offset_mm": 50}));
    let notch = notch.created[0].to_string();
    h.round_trip(json!({"type": "notch.move", "notch": notch, "offset_mm": 60}));
    h.round_trip(json!({"type": "point.remove", "point": new_point}));
    h.round_trip(json!({"type": "notch.remove", "notch": notch}));

    let line = h.round_trip(json!({"type": "internal_line.add", "panel": panel,
    "start": [200, 100], "segments": [
        {"kind": "line", "to": [200, 300]},
        {"kind": "cubic", "c1": [200, 400], "c2": [250, 450], "to": [300, 500]}
    ]}));
    let line = line.created[0].to_string();

    let copy = h.round_trip(json!({"type": "panel.mirror", "panel": panel}));
    let copy = copy.created[0].to_string();
    h.round_trip(json!({"type": "panel.unlink", "panel": copy}));
    h.round_trip(json!({"type": "internal_line.remove", "line": line}));
    h.round_trip(json!({"type": "edge.set_curve", "edge": edge, "curve": {"kind": "line"}}));
    h.round_trip(json!({"type": "panel.delete", "panel": copy, "confirm": true}));
    // The jacket's piece goes with it.
    h.round_trip(json!({"type": "garment.remove", "garment": jacket, "confirm": true}));
    let path = h.round_trip(json!({
        "type": "panel.create_path", "name": "Bodice", "start": [2000, 0], "segments": [
            {"kind": "line", "to": [2300, 0]},
            {"kind": "cubic", "c1": [2350, 200], "c2": [2250, 400], "to": [2300, 600]},
            {"kind": "line", "to": [2000, 600]}
        ]
    }));
    // The closing edge back to the start is added.
    assert_eq!(path.created.len(), 1 + 4 + 4);
    let cut = h.round_trip(json!({"type": "panel.cut", "panel": path.created[0],
        "start": [2150, 650], "segments": [{"kind": "cubic", "c1": [2100, 400], "c2": [2200, 200], "to": [2150, -50]}]}));
    assert_eq!(cut.created.len(), 2);
    let imported = h.round_trip(json!({"type": "pattern.import", "pieces": [{
        "name": "Pocket", "start": [0, 0], "seam_allowance_mm": 10, "grain_angle_deg": 0,
        "segments": [
            {"kind": "line", "to": [150, 0]},
            {"kind": "cubic", "c1": [160, 60], "c2": [150, 120], "to": [150, 180]},
            {"kind": "line", "to": [0, 180]},
            {"kind": "line", "to": [0, 0]}
        ],
        "notches": [[75, 1], [0, 90]],
        "internal_lines": [{"start": [20, 160], "segments": [{"kind": "line", "to": [130, 160]}]}]
    }]}));
    // Panel, 4 points, 4 edges, 2 notches, 1 line.
    assert_eq!(imported.created.len(), 1 + 4 + 4 + 2 + 1);

    let registered: BTreeSet<_> = Command::registry().iter().map(|i| i.name).collect();
    let missing: Vec<_> = registered.difference(&h.covered).collect();
    assert!(
        missing.is_empty(),
        "commands without a round-trip test: {missing:?}"
    );
}
