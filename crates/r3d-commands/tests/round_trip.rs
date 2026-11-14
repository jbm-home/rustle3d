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

    let rect = h.round_trip(json!({
        "type": "panel.create_rect", "name": "Front", "origin": [10, 20], "width": 400, "height": 600
    }));
    assert_eq!(rect.created.len(), 1 + 4 + 4);
    let panel = rect.created[0].to_string();
    let point = rect.created[2].to_string();
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
    h.round_trip(json!({"type": "panel.translate", "panel": panel, "delta": [100, -50]}));
    h.round_trip(json!({"type": "edge.set_curve", "edge": edge, "curve": {"kind": "line"}}));
    h.round_trip(json!({"type": "panel.delete", "panel": panel, "confirm": true}));

    let registered: BTreeSet<_> = Command::registry().iter().map(|i| i.name).collect();
    let missing: Vec<_> = registered.difference(&h.covered).collect();
    assert!(
        missing.is_empty(),
        "commands without a round-trip test: {missing:?}"
    );
}
