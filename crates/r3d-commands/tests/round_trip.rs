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
    h.round_trip(json!({"type": "avatar.set_measures",
        "measures": {"neck": 37.5, "arm_length": 58}}));
    h.round_trip(json!({"type": "avatar.set_measures", "measures": {"neck": null}}));
    h.round_trip(json!({"type": "avatar.remove"}));

    // An imported body: the built-in one written as an OBJ, read into
    // memory as `avatar.import` does.
    let form = r3d_commands::drape::Form::of_body(
        &serde_json::from_value(body.clone()).unwrap(),
        &Default::default(),
    )
    .unwrap();
    let mut obj = String::new();
    for p in &form.positions {
        obj += &format!(
            "v {} {} {}
",
            p.x, p.y, p.z
        );
    }
    for t in &form.triangles {
        obj += &format!(
            "f {} {} {}
",
            t[0] + 1,
            t[1] + 1,
            t[2] + 1
        );
    }
    let loaded =
        r3d_commands::bodies::register(obj.as_bytes(), r3d_io::model::ModelFormat::Obj).unwrap();
    h.round_trip(
        json!({"type": "avatar.import_mesh", "file": "avatars/body.obj",
        "hash": loaded.hash}),
    );
    h.round_trip(json!({"type": "avatar.orient", "turn_deg": 10}));
    h.round_trip(json!({"type": "avatar.set_point", "point": "waist", "at_mm": [0, 1000, 0]}));
    h.round_trip(json!({"type": "avatar.guess_points"}));
    h.round_trip(json!({"type": "avatar.remove_mesh"}));
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
    h.round_trip(json!({"type": "panel.rotate", "panel": panel, "angle_deg": 30}));
    h.round_trip(
        json!({"type": "panel.scale", "panel": panel, "factor": [1.1, 0.9], "centre": [0, 0]}),
    );
    h.round_trip(json!({"type": "panel.set_grain", "panel": panel, "angle_deg": 45}));
    h.round_trip(json!({"type": "panel.set_seam_allowance", "panel": panel, "mm": 15}));
    h.round_trip(json!({"type": "panel.place", "panel": panel, "angle_deg": 0, "top_cm": 104}));
    h.round_trip(json!({"type": "garment.set_fabric", "fabric": "denim"}));
    h.round_trip(json!({"type": "garment.set_color", "color": "#335577"}));
    h.round_trip(
        json!({"type": "garment.set_fabric_details", "weight_gsm": 210,
        "width_mm": 1450, "price_per_m": 12.5}),
    );
    h.round_trip(json!({"type": "garment.set_notes", "notes": "Press seams open.\n"}));
    h.round_trip(json!({"type": "garment.set_print", "print": {"builtin": "stripes"}}));
    h.round_trip(json!({"type": "garment.set_print",
        "print": {"image": "textures/liberty.jpg"}, "width_mm": 320, "angle_deg": 30}));
    h.round_trip(json!({"type": "panel.set_print_placement", "panel": panel,
        "offset_mm": [12.5, -4], "angle_deg": 90}));
    h.round_trip(json!({"type": "garment.remove_print"}));
    h.round_trip(json!({"type": "photo.set_scene", "scene": {
        "light": {"kind": "environment", "name": "pav"}, "turn_deg": 30, "brightness": 1.2,
        "camera": {"target_mm": [0, 900, 0], "yaw_deg": 20, "pitch_deg": 5,
            "distance_mm": 3000, "focal_mm": 50},
        "size": [1080, 1350]}}));
    h.round_trip(json!({"type": "photo.set_scene", "scene": null}));
    let jacket = h.round_trip(json!({"type": "garment.add", "name": "Jacket",
        "fabric": "wool", "color": "#223344"}));
    let jacket = jacket.created[0].to_string();
    h.round_trip(json!({"type": "garment.rename", "garment": jacket, "name": "Coat"}));
    h.round_trip(json!({"type": "panel.set_garment", "panel": panel, "garment": jacket}));
    h.round_trip(json!({"type": "garment.set_layer", "garment": jacket, "layer": 0}));
    h.round_trip(json!({"type": "garment.set_draped", "garment": jacket, "draped": false}));
    h.round_trip(json!({"type": "garment.set_draped", "garment": jacket, "draped": true}));
    let poly_edge = poly.created[5].to_string();
    let other_edge = rect.created[7].to_string();
    let seam = h.round_trip(json!({"type": "seam.add",
        "a": {"edge": poly_edge}, "b": {"edge": other_edge, "reversed": true}}));
    let seam = seam.created[0].to_string();
    h.round_trip(json!({"type": "seam.flip", "seam": seam}));
    h.round_trip(json!({"type": "seam.set_gathered", "seam": seam, "gathered": true}));
    h.round_trip(json!({"type": "seam.set_ease", "seam": seam, "mm": 8}));
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
    let poly_panel = poly.created[0].to_string();
    let button = h.round_trip(json!({"type": "button.add", "panel": poly_panel,
        "at": [100, 300], "hole_panel": poly_panel, "hole_at": [250, 300]}));
    let button = button.created[0].to_string();
    h.round_trip(
        json!({"type": "button.set", "button": button, "diameter_mm": 15,
        "hole_angle_deg": 90}),
    );
    h.round_trip(json!({"type": "button.set_closed", "buttons": [button], "closed": false}));
    h.round_trip(json!({"type": "button.remove", "button": button}));
    let zipper = h.round_trip(json!({"type": "zipper.add",
        "a": {"edge": poly.created[6].to_string(), "to_mm": 200},
        "b": {"edge": poly.created[8].to_string(), "from_mm": 100}}));
    let zipper = zipper.created[0].to_string();
    h.round_trip(json!({"type": "zipper.set_open", "zipper": zipper, "open_mm": 80}));
    h.round_trip(json!({"type": "zipper.remove", "zipper": zipper}));
    let elastic = h.round_trip(json!({"type": "elastic.add",
        "side": {"edge": poly.created[7].to_string(), "from_mm": 20, "to_mm": 120},
        "ratio": 0.6}));
    let elastic = elastic.created[0].to_string();
    h.round_trip(
        json!({"type": "elastic.set", "elastic": elastic, "relaxed_mm": 70,
        "width_mm": 20}),
    );
    h.round_trip(json!({"type": "elastic.remove", "elastic": elastic}));
    let topstitch = h.round_trip(json!({"type": "topstitch.add",
        "side": {"edge": poly.created[7].to_string()}, "rows": 2}));
    let topstitch = topstitch.created[0].to_string();
    h.round_trip(json!({"type": "topstitch.set", "topstitch": topstitch,
        "color": "#203040", "stitch_mm": 4}));
    h.round_trip(json!({"type": "topstitch.remove", "topstitch": topstitch}));
    let interfacing = h.round_trip(json!({"type": "interfacing.add", "panel": poly_panel,
        "area": {"kind": "band", "edges": [poly.created[7].to_string()], "width_mm": 30}}));
    let interfacing = interfacing.created[0].to_string();
    h.round_trip(
        json!({"type": "interfacing.set", "interfacing": interfacing,
        "weight": "heavy", "width_mm": 40}),
    );
    h.round_trip(json!({"type": "interfacing.remove", "interfacing": interfacing}));
    let quilting = h.round_trip(json!({"type": "quilting.add", "panel": poly_panel,
        "spacing_mm": 60}));
    let quilting = quilting.created[0].to_string();
    h.round_trip(json!({"type": "quilting.set", "quilting": quilting,
        "pattern": "channels", "loft_mm": 12}));
    h.round_trip(json!({"type": "quilting.remove", "quilting": quilting}));
    let shirring = h.round_trip(json!({"type": "elastic.shirr", "panel": poly_panel,
        "edge": poly.created[5].to_string(), "rows": 3}));
    // A shirring row's elastic, with its line.
    h.round_trip(json!({"type": "elastic.remove",
        "elastic": shirring.created[1].to_string()}));

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

    h.round_trip(json!({"type": "grading.use_chart", "chart": "women",
        "from": "36", "to": "44", "base": "40"}));
    let corner = imported.created[2].to_string();
    h.round_trip(json!({"type": "point.set_grade", "point": corner, "step": [5, 0]}));
    h.round_trip(json!({"type": "point.set_grade", "point": corner,
        "offsets": {"44": [12, 3]}}));
    h.round_trip(json!({"type": "grading.show_size", "size": "44"}));
    // A new base redraws the pattern in that size.
    h.round_trip(json!({"type": "grading.set_sizes", "base": "38", "sizes": [
        {"name": "38", "bust_cm": -4}, {"name": "40"}, {"name": "42", "bust_cm": 4}]}));
    h.round_trip(json!({"type": "grading.clear", "confirm": true}));

    // A parametric pattern (spec 28), drafted again, and a multisize
    // measurement file.
    let parametric = h.round_trip(json!({"type": "pattern.import_parametric",
        "file": "imports/skirt.val",
        "pattern": include_str!("data/parametric/skirt.val"),
        "measurements": include_str!("data/parametric/body.vit")}));
    h.round_trip(
        json!({"type": "garment.redraft", "garment": parametric.created[0],
        "body": {"masculinity": 0, "height_cm": 168, "bust_cm": 92, "waist_cm": 80,
        "hips_cm": 104}}),
    );
    h.round_trip(json!({"type": "avatar.use_measurements",
        "measurements": include_str!("data/parametric/sizes.vst")}));
    h.round_trip(json!({"type": "grading.clear", "confirm": true}));
    let copy = r3d_commands::step::parse_query(
        json!({"type": "panel.copy", "panels": [imported.created[0]]}),
    )
    .unwrap();
    let clip = h.session.query(&copy).unwrap();
    h.round_trip(json!({"type": "panel.paste", "clip": clip}));

    // Constraints and dimensions (spec 19).
    let quad = h.round_trip(json!({"type": "panel.create_polygon",
        "points": [[5000, 0], [5300, 10], [5300, 400], [5000, 400]]}));
    let (q_pts, q_edges): (Vec<String>, Vec<String>) = (
        quad.created[1..5].iter().map(ToString::to_string).collect(),
        quad.created[5..9].iter().map(ToString::to_string).collect(),
    );
    h.round_trip(json!({"type": "constraint.add",
        "constraint": {"kind": "horizontal", "edge": q_edges[0]}}));
    let len = h.round_trip(json!({"type": "constraint.add", "measure": true,
        "constraint": {"kind": "length", "edge": q_edges[0], "mm": 1}}));
    let len = len.created[0].to_string();
    h.round_trip(json!({"type": "constraint.set_value", "constraint": len, "value": 320}));
    h.round_trip(json!({"type": "variable.set", "name": "ease", "formula": "2 cm"}));
    h.round_trip(json!({"type": "constraint.set_formula", "constraint": len,
        "formula": "ease * 15"}));
    h.round_trip(json!({"type": "constraint.rename", "constraint": len, "name": "side"}));
    h.round_trip(json!({"type": "constraint.place_label", "constraint": len,
        "offset": [10, -20]}));
    h.round_trip(json!({"type": "constraint.rename", "constraint": len}));
    h.round_trip(json!({"type": "constraint.remove", "constraint": len}));
    h.round_trip(json!({"type": "variable.remove", "name": "ease"}));
    h.round_trip(json!({"type": "point.set_smooth", "point": q_pts[2], "smooth": true}));
    h.round_trip(json!({"type": "panel.unfold", "panel": quad.created[0], "edge": q_edges[3]}));
    h.round_trip(json!({"type": "panel.unlink", "panel": quad.created[0]}));
    let guide = h.round_trip(json!({"type": "guide.add", "a": [0, -100], "b": [600, -100]}));
    let guide = guide.created[0].to_string();
    h.round_trip(json!({"type": "guide.move", "guide": guide, "a": [0, -120], "b": [600, -120]}));
    h.round_trip(json!({"type": "guide.remove", "guide": guide}));
    h.round_trip(json!({"type": "panel.create_ellipse", "centre": [7000, 0], "radii": [80, 50]}));
    let sector = h.round_trip(json!({"type": "panel.create_sector", "centre": [8000, 0],
        "inner_length_mm": 350, "length_mm": 600, "sweep_deg": 180}));
    h.round_trip(
        json!({"type": "edge.set_arc", "edge": sector.created.last().unwrap(), "bulge_mm": 40}),
    );

    // Construction details (spec 21).
    let skirt = h.round_trip(json!({"type": "panel.create_rect", "origin": [9000, 0],
        "width": 400, "height": 600}));
    let top = skirt.created[7].to_string();
    h.round_trip(
        json!({"type": "dart.add", "edge": top, "at_mm": 300, "width_mm": 20,
        "length_mm": 100}),
    );
    h.round_trip(
        json!({"type": "pleat.add", "edge": top, "at_mm": 50, "depth_mm": 20,
        "kind": "box", "stitched_mm": 50}),
    );
    let hole = h.round_trip(json!({"type": "panel.add_hole", "panel": skirt.created[0],
        "start": [9100, 100], "segments": [{"kind": "line", "to": [9150, 100]},
        {"kind": "line", "to": [9150, 150]}]}));
    h.round_trip(json!({"type": "panel.remove_hole", "edge": hole.created[3]}));
    let yoke = h.round_trip(json!({"type": "panel.create_rect", "origin": [10000, 0],
        "width": 300, "height": 200}));
    let patch = h.round_trip(json!({"type": "panel.create_rect", "origin": [10250, 50],
        "width": 100, "height": 100}));
    h.round_trip(json!({"type": "panel.boolean", "panel": yoke.created[0],
        "with": patch.created[0], "op": "difference"}));

    // A saved pattern (spec 32): this project's garments added again.
    let pattern: Value =
        serde_json::from_str(&r3d_core::file::to_json(h.session.project()).unwrap()).unwrap();
    h.round_trip(json!({"type": "pattern.add", "pattern": pattern}));

    // An object (spec 36): the body goes, then the project is a garment
    // again.
    h.round_trip(json!({"type": "object.set"}));
    h.round_trip(json!({"type": "object.set", "stuffing": 0.8}));
    h.round_trip(json!({"type": "object.clear"}));

    let registered: BTreeSet<_> = Command::registry().iter().map(|i| i.name).collect();
    let missing: Vec<_> = registered.difference(&h.covered).collect();
    assert!(
        missing.is_empty(),
        "commands without a round-trip test: {missing:?}"
    );
}
