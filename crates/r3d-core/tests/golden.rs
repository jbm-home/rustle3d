//! Golden-file tests for the project format.
//!
//! Each schema version keeps sample files under `tests/golden/v{N}/`. Old
//! files are never edited: they prove that migration keeps working.
//! Regenerate the *current* version's files with `UPDATE_GOLDEN=1 cargo test`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // test code

use std::fs;
use std::path::PathBuf;

use glam::DVec2;
use r3d_core::file::{self, SCHEMA_VERSION};
use r3d_core::{
    Along, Around, Avatar, Body, Constraint, Edge, EdgeCurve, Fabric, Garment, Grading, Guide,
    InternalLine, Notch, Panel, PathSegment, Pin, Placement, Point, Project, Seam, SeamSide, Size,
    Symmetry, TemplateRef, Transaction,
};

fn golden_dir(version: u32) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("tests/golden/v{version}"))
}

/// A small project exercising every entity kind of the current schema.
fn sample() -> Project {
    let mut p = Project::new("Golden sample");
    let mut tx = Transaction::begin(&mut p);
    let garment = tx.project().default_garment().unwrap();
    let panel = tx.new_panel_id();
    let corners = [(0.0, 0.0), (400.0, 0.0), (380.0, 600.0), (20.0, 600.0)];
    let pts: Vec<_> = corners
        .iter()
        .map(|&(x, y)| {
            let id = tx.new_point_id();
            // The right side is graded: 10 mm outward per size.
            let grade = if x > 200.0 {
                vec![DVec2::new(-10.0, 0.0), DVec2::ZERO, DVec2::new(10.0, 0.0)]
            } else {
                Vec::new()
            };
            tx.put_point(
                id,
                Point {
                    panel,
                    pos: DVec2::new(x, y),
                    grade,
                },
            );
            id
        })
        .collect();
    let mut contour = Vec::new();
    for i in 0..pts.len() {
        let id = tx.new_edge_id();
        let curve = if i == 2 {
            EdgeCurve::Cubic {
                c1: DVec2::new(260.0, 615.0),
                c2: DVec2::new(140.0, 615.0),
            }
        } else {
            EdgeCurve::Line
        };
        // The curved top's handles are graded with its right end.
        let grade = if i == 2 {
            vec![
                [DVec2::new(-5.0, 0.0), DVec2::ZERO],
                [DVec2::ZERO; 2],
                [DVec2::new(5.0, 0.0), DVec2::ZERO],
            ]
        } else {
            Vec::new()
        };
        tx.put_edge(
            id,
            Edge {
                panel,
                start: pts[i],
                end: pts[(i + 1) % pts.len()],
                curve,
                grade,
            },
        );
        contour.push(id);
    }
    tx.put_panel(
        panel,
        Panel {
            name: "Front".into(),
            garment,
            contour: contour.clone(),
            grain_angle_deg: 90.0,
            seam_allowance_mm: 10.0,
            mirror_of: None,
            placement: Some(Placement {
                around: Around::Body,
                angle_deg: 0.0,
                top_cm: 104.0,
                offset_mm: 30.0,
                centre_x_mm: Some(200.0),
            }),
            symmetry: Some(Symmetry {
                origin: DVec2::new(200.0, 0.0),
                dir: DVec2::new(0.0, 1.0),
                start: pts[0],
                end: pts[2],
                points: vec![[pts[1], pts[3]]],
                edges: vec![[contour[0], contour[3]], [contour[1], contour[2]]],
            }),
        },
    );
    let outer = tx.new_garment_id();
    tx.put_garment(
        outer,
        Garment {
            name: "Jacket".into(),
            fabric: Fabric::Wool,
            color: Some("#334455".into()),
            layer: 1,
            template: Some(TemplateRef {
                name: "shirt".into(),
                sizes: [("length_cm".to_owned(), 70.0)].into(),
            }),
        },
    );
    let pin = tx.new_pin_id();
    tx.put_pin(
        pin,
        Pin {
            panel,
            at: DVec2::new(10.0, 20.0),
            position: glam::DVec3::new(0.05, 1.1, 0.12),
        },
    );
    let seam = tx.new_seam_id();
    tx.put_seam(
        seam,
        Seam {
            a: SeamSide {
                edge: contour[1],
                reversed: false,
            },
            b: SeamSide {
                edge: contour[3],
                reversed: true,
            },
            gathered: true,
        },
    );
    let notch = tx.new_notch_id();
    tx.put_notch(
        notch,
        Notch {
            edge: contour[1],
            offset_mm: 180.0,
        },
    );
    let line = tx.new_internal_line_id();
    tx.put_internal_line(
        line,
        InternalLine {
            panel,
            start: DVec2::new(200.0, 600.0),
            segments: vec![
                PathSegment::Line {
                    to: DVec2::new(200.0, 480.0),
                },
                PathSegment::Cubic {
                    c1: DVec2::new(200.0, 400.0),
                    c2: DVec2::new(180.0, 350.0),
                    to: DVec2::new(160.0, 300.0),
                },
            ],
        },
    );
    tx.set_avatar(Some(Avatar {
        body: Body {
            masculinity: 0.0,
            height_cm: 168.0,
            bust_cm: 90.0,
            waist_cm: 73.0,
            hips_cm: 98.0,
        },
        preset: Some("Women M".into()),
    }));
    let size = |name: &str, bust_cm: f64| Size {
        name: name.into(),
        height_cm: 0.0,
        bust_cm,
        waist_cm: bust_cm,
        hips_cm: bust_cm,
    };
    tx.set_grading(Some(Grading {
        sizes: vec![size("38", -4.0), size("40", 0.0), size("42", 4.0)],
        base: 1,
        shown: 2,
    }));
    for c in [
        Constraint::Horizontal { edge: contour[0] },
        Constraint::Fixed { point: pts[0] },
        Constraint::Distance {
            a: pts[0],
            b: pts[1],
            mm: 400.0,
            along: Along::Horizontal,
            formula: Some("bust / 2 - ease".into()),
        },
        Constraint::Angle {
            a: contour[0],
            b: contour[1],
            deg: 92.0,
            formula: None,
        },
    ] {
        let id = tx.new_constraint_id();
        tx.put_constraint(id, c);
    }
    tx.set_variables([("ease".to_owned(), "5 cm".to_owned())].into());
    let guide = tx.new_guide_id();
    tx.put_guide(
        guide,
        Guide {
            a: DVec2::new(-50.0, 300.0),
            b: DVec2::new(450.0, 300.0),
        },
    );
    tx.commit();
    p
}

#[test]
fn current_version_matches_golden() {
    let project = sample();
    project.check_integrity().unwrap();
    let json = file::to_json(&project).unwrap();
    let path = golden_dir(SCHEMA_VERSION).join("sample.json");
    if std::env::var_os("UPDATE_GOLDEN").is_some() {
        fs::write(&path, &json).unwrap();
    }
    let expected = fs::read_to_string(&path).unwrap();
    assert_eq!(
        json, expected,
        "golden file differs; run with UPDATE_GOLDEN=1 if intended"
    );
}

#[test]
fn every_golden_file_loads_and_resaves_identically_at_current_version() {
    for version in 1..=SCHEMA_VERSION {
        for entry in fs::read_dir(golden_dir(version)).unwrap() {
            let path = entry.unwrap().path();
            let text = fs::read_to_string(&path).unwrap();
            let loaded =
                file::from_json(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            let resaved = file::to_json(&loaded.project).unwrap();
            if version == SCHEMA_VERSION {
                assert_eq!(resaved, text, "{} is not byte-stable", path.display());
                assert_eq!(loaded.migrated_from, None);
            } else {
                assert_eq!(loaded.migrated_from, Some(version));
            }
            // Saving then loading again must be lossless.
            assert_eq!(file::from_json(&resaved).unwrap().project, loaded.project);
        }
    }
}
