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
use r3d_core::{Edge, EdgeCurve, Panel, Point, Project, Transaction};

fn golden_dir(version: u32) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("tests/golden/v{version}"))
}

/// A small project with one straight-edged panel and one curved edge.
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
            contour,
            grain_angle_deg: 90.0,
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
            }
            // Saving then loading again must be lossless.
            assert_eq!(file::from_json(&resaved).unwrap().project, loaded.project);
        }
    }
}
