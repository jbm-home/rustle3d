#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // test code

use r3d_commands::step::parse_command;
use r3d_commands::{CommandError, Invalid, Query, Session, Step};
use r3d_core::{Localizable, Project};
use serde_json::{Value, json};

fn session_with_rect() -> (Session, String, String) {
    let mut s = Session::new(Project::new("t"));
    let cmd =
        parse_command(json!({"type": "panel.create_rect", "width": 100, "height": 200})).unwrap();
    let out = s.execute(&cmd).unwrap();
    (s, out.created[0].to_string(), out.created[1].to_string())
}

fn run(s: &mut Session, step: Value) -> Result<Value, CommandError> {
    let step = Step::from_value(step)?;
    s.run_step(&step)
}

#[test]
fn invalid_input_changes_nothing() {
    let (mut s, _, _) = session_with_rect();
    let before = s.project().clone();
    let cases = [
        (
            json!({"type": "panel.create_rect", "width": -1, "height": 10}),
            "invalid.not_positive",
        ),
        (
            json!({"type": "panel.create_polygon", "points": [[0, 0], [1, 1]]}),
            "invalid.too_few_points",
        ),
        (
            json!({"type": "panel.create_polygon", "points": [[0, 0], [10, 10], [10, 0], [0, 10]]}),
            "invalid.not_simple",
        ),
        (
            json!({"type": "panel.create_polygon", "points": [[0, 0], [5, 0], [10, 0]]}),
            "invalid.not_simple",
        ),
        (
            json!({"type": "panel.rename", "panel": "panel:999", "name": "x"}),
            "not_found",
        ),
        (
            json!({"type": "panel.rename", "panel": "point:3", "name": "x"}),
            "malformed",
        ),
        (
            json!({"type": "panel.create_rect", "width": 1, "height": 1, "colour": "red"}),
            "malformed",
        ),
        (json!({"type": "panel.explode"}), "unknown_type"),
        (
            json!({"type": "panel.delete", "panel": "panel:2"}),
            "confirmation_required",
        ),
    ];
    for (step, code) in cases {
        let err = run(&mut s, step.clone()).unwrap_err();
        assert_eq!(err.message().code, code, "{step}: {err}");
        assert_eq!(s.project(), &before, "{step} modified the document");
    }
}

#[test]
fn batch_is_atomic_and_one_undo_step() {
    let (mut s, panel, _) = session_with_rect();
    let before = s.project().clone();
    let err = run(
        &mut s,
        json!({"type": "batch", "commands": [
            {"type": "panel.rename", "panel": panel, "name": "A"},
            {"type": "panel.create_rect", "width": 0, "height": 1},
        ]}),
    )
    .unwrap_err();
    assert!(
        matches!(err, CommandError::AtStep { index: 1, .. }),
        "{err:?}"
    );
    assert_eq!(err.message().args.get("step").cloned(), Some("1".into()));
    assert_eq!(s.project(), &before);

    run(
        &mut s,
        json!({"type": "batch", "commands": [
            {"type": "panel.rename", "panel": panel, "name": "A"},
            {"type": "panel.create_rect", "width": 1, "height": 1},
        ]}),
    )
    .unwrap();
    assert_eq!(s.project().panels().len(), 2);
    run(&mut s, json!({"type": "history.undo"})).unwrap();
    assert_eq!(s.project(), &before);
}

#[test]
fn consecutive_moves_merge_into_one_undo_step() {
    let (mut s, _, point) = session_with_rect();
    let before = s.project().clone();
    for x in 1..=50 {
        run(
            &mut s,
            json!({"type": "point.move", "point": point, "to": [x, 0]}),
        )
        .unwrap();
    }
    assert_eq!(s.history().undo_len(), 2); // create + merged moves
    s.undo().unwrap();
    assert_eq!(s.project(), &before);

    // A seal splits the run.
    run(
        &mut s,
        json!({"type": "point.move", "point": point, "to": [5, 0]}),
    )
    .unwrap();
    s.seal_history();
    run(
        &mut s,
        json!({"type": "point.move", "point": point, "to": [6, 0]}),
    )
    .unwrap();
    assert_eq!(s.history().undo_len(), 3);
}

#[test]
fn undo_redo_errors_when_empty() {
    let mut s = Session::new(Project::new("t"));
    assert_eq!(s.undo(), Err(CommandError::NothingToUndo));
    assert_eq!(s.redo(), Err(CommandError::NothingToRedo));
}

#[test]
fn new_command_clears_redo() {
    let (mut s, _, _) = session_with_rect();
    s.undo().unwrap();
    assert_eq!(s.history().redo_len(), 1);
    run(
        &mut s,
        json!({"type": "panel.create_rect", "width": 5, "height": 5}),
    )
    .unwrap();
    assert_eq!(s.history().redo_len(), 0);
}

#[test]
fn queries_measure_correctly() {
    let (mut s, panel, _) = session_with_rect();
    let summary = run(&mut s, json!({"type": "project.summary"})).unwrap();
    assert_eq!(summary["counts"]["panels"], 1);
    assert_eq!(summary["panels"][0]["area_mm2"], 20000.0);
    assert_eq!(summary["panels"][0]["perimeter_mm"], 600.0);

    let got = run(&mut s, json!({"type": "panel.get", "panel": panel})).unwrap();
    let edges = got["edges"].as_array().unwrap();
    assert_eq!(edges.len(), 4);
    assert_eq!(edges[0]["length_mm"], 100.0);
    let a = edges[0]["start"]["id"].clone();
    let b = edges[1]["end"]["id"].clone();
    let d = run(&mut s, json!({"type": "measure.distance", "a": a, "b": b})).unwrap();
    assert_eq!(d["distance_mm"], 223.607); // diagonal of 100 × 200

    let err = run(&mut s, json!({"type": "measure.distance", "a": a, "b": a})).unwrap_err();
    assert_eq!(err, CommandError::invalid("b", Invalid::SamePoint));
}

#[test]
fn moving_a_point_can_make_a_panel_non_simple_and_queries_report_it() {
    let (mut s, panel, point) = session_with_rect();
    // Drag the bottom-left corner past the right side: the closing edge now
    // crosses the right edge (bow-tie).
    run(
        &mut s,
        json!({"type": "point.move", "point": point, "to": [200, 100]}),
    )
    .unwrap();
    let got = run(&mut s, json!({"type": "panel.get", "panel": panel})).unwrap();
    assert_eq!(got["is_simple"], false);
}

#[test]
fn command_and_query_names_are_distinct_and_well_formed() {
    let mut names: Vec<_> = r3d_commands::Command::registry()
        .iter()
        .map(|i| i.name)
        .collect();
    names.extend(Query::registry().iter().map(|i| i.name));
    names.extend(r3d_commands::Action::registry().iter().map(|i| i.name));
    names.extend([
        r3d_commands::step::UNDO,
        r3d_commands::step::REDO,
        r3d_commands::step::LIST,
        r3d_commands::step::GO_TO,
        r3d_commands::step::BATCH,
    ]);
    let unique: std::collections::BTreeSet<_> = names.iter().collect();
    assert_eq!(unique.len(), names.len(), "duplicate operation name");
    for n in names {
        assert!(
            n.chars()
                .all(|c| c.is_ascii_lowercase() || c == '.' || c == '_'),
            "{n}"
        );
    }
}

#[test]
fn scripts_report_the_failing_step_and_resolve_references() {
    let mut s = Session::new(Project::new("t"));
    let script =
        r3d_commands::Script::parse(r#"[{"type":"project.summary"},{"type":"nope"}]"#).unwrap();
    let err = script.run(|step| s.run_step(step)).unwrap_err();
    assert_eq!(err.index, 1);
    assert_eq!(err.message().code, "unknown_type");
    assert_eq!(
        err.message().args.get("step").map(String::as_str),
        Some("1")
    );

    let script = r3d_commands::Script::parse(
        r#"[
            {"type": "panel.create_rect", "width": 100, "height": 200},
            {"type": "panel.rename", "panel": "$0.created[0]", "name": "Front"},
            {"type": "panel.get", "panel": "$0.created[0]"},
            {"type": "point.move", "point": "$2.edges[1].start.id", "to": [120, 0]}
        ]"#,
    )
    .unwrap();
    let results = script.run(|step| s.run_step(step)).unwrap();
    assert_eq!(results[2]["name"], "Front");
    assert_eq!(s.project().panels().len(), 1);

    assert!(r3d_commands::Script::parse("{}").is_err());
    let script = r3d_commands::Script::parse("[1]").unwrap();
    assert!(script.run(|step| s.run_step(step)).is_err());
}

#[test]
fn history_lists_its_steps_and_jumps_to_any() {
    let (mut s, panel, point) = session_with_rect();
    let start = s.project().clone();
    run(
        &mut s,
        json!({"type": "panel.rename", "panel": panel, "name": "Front"}),
    )
    .unwrap();
    run(
        &mut s,
        json!({"type": "batch", "label": "panel.paste", "commands": [
            {"type": "point.move", "point": point, "to": [5, 5]},
        ]}),
    )
    .unwrap();
    let end = s.project().clone();
    let list = run(&mut s, json!({"type": "history.list"})).unwrap();
    let labels: Vec<_> = list["steps"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["label"].as_str().unwrap())
        .collect();
    assert_eq!(labels, ["panel.create_rect", "panel.rename", "panel.paste"]);
    assert_eq!(list["at"], 3);

    // Back to the start, then to the end: the steps stay listed.
    let back = run(&mut s, json!({"type": "history.go_to", "at": 0})).unwrap();
    assert_eq!(back["at"], 0);
    assert_eq!(s.project(), &Project::new("t"));
    assert_eq!(s.history().len(), 3);
    run(&mut s, json!({"type": "history.go_to", "at": 1})).unwrap();
    assert_eq!(s.project().panels().len(), start.panels().len());
    let forward = run(&mut s, json!({"type": "history.go_to", "at": 3})).unwrap();
    assert_eq!(s.project(), &end);
    // The panel the redone steps changed, to select.
    assert_eq!(forward["panels"], json!([panel]));
    // Where it already is: nothing to do, no error.
    run(&mut s, json!({"type": "history.go_to", "at": 3})).unwrap();
    let err = run(&mut s, json!({"type": "history.go_to", "at": 4})).unwrap_err();
    assert_eq!(err.message().code, "invalid.range");
    let err = run(&mut s, json!({"type": "history.go_to"})).unwrap_err();
    assert_eq!(err.message().code, "malformed");
}

#[test]
fn undo_names_the_step_and_its_panels() {
    let (mut s, panel, _) = session_with_rect();
    run(
        &mut s,
        json!({"type": "panel.rename", "panel": panel, "name": "Front"}),
    )
    .unwrap();
    let undone = run(&mut s, json!({"type": "history.undo"})).unwrap();
    assert_eq!(undone, json!({"undone": "panel.rename", "panels": [panel]}));
    // Undoing the creation: the panel is gone, nothing to select.
    let undone = run(&mut s, json!({"type": "history.undo"})).unwrap();
    assert_eq!(undone, json!({"undone": "panel.create_rect", "panels": []}));
    let redone = run(&mut s, json!({"type": "history.redo"})).unwrap();
    assert_eq!(redone["panels"], json!([panel]));
}
