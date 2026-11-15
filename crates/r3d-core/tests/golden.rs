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
    Avatar, Body, Edge, EdgeCurve, InternalLine, Notch, Panel, PathSegment, Placement, Point,
    Project, Seam, SeamSide, Transaction,
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
            tx.put_point(
                id,
                Point {
                    panel,
                    pos: DVec2::new(x, y),
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
        tx.put_edge(
            id,
            Edge {
                panel,
                start: pts[i],
                end: pts[(i + 1) % pts.len()],
                curve,
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
                angle_deg: 0.0,
                top_cm: 104.0,
                offset_mm: 30.0,
            }),
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
