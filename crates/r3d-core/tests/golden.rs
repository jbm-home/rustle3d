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
    Along, Around, Avatar, AvatarMesh, Body, BodyPoint, Button, Buttonhole, Constraint, Edge,
    EdgeCurve, EdgeEnd, Fabric, FabricDetails, Garment, Grading, Guide, InternalLine, Notch,
    ObjectSetup, Panel, PathSegment, PatternSource, PhotoCamera, PhotoLight, PhotoSetup, Pin,
    Placement, Point, Print, PrintPlacement, PrintSource, Project, Seam, SeamSide, Size,
    SourcePiece, Symmetry, TemplateRef, Transaction, UpAxis, Zipper,
};
use r3d_core::{
    Detail, Elastic, Interfacing, InterfacingArea, InterfacingFabric, InterfacingWeight,
    QuiltPattern, Quilting, Thread, Topstitch,
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
            holes: Vec::new(),
            print: PrintPlacement {
                offset_mm: DVec2::new(10.0, -5.0),
                angle_deg: 0.0,
            },
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
            fabric_details: FabricDetails {
                weight_gsm: Some(310.0),
                width_mm: Some(1450.0),
                price_per_m: Some(18.5),
            },
            notes: "Topstitch the collar 6 mm from the edge.\nPress seams open.".into(),
            print: Some(Print {
                source: PrintSource::Image("textures/tweed.jpg".into()),
                width_mm: 320.0,
                height_mm: None,
                angle_deg: 45.0,
            }),
            source: None,
            // Taken out of the drape (v26).
            draped: false,
        },
    );
    // The first garment was imported from a parametric pattern.
    let first = tx.project().garments().get(&garment).cloned().unwrap();
    tx.put_garment(
        garment,
        Garment {
            source: Some(PatternSource {
                file: "imports/skirt.val".into(),
                pattern: "<pattern><version>1.2.1</version></pattern>".into(),
                measurements: Some("<vit><unit>cm</unit></vit>".into()),
                body: Some(Body {
                    masculinity: 0.0,
                    height_cm: 168.0,
                    bust_cm: 88.0,
                    waist_cm: 70.0,
                    hips_cm: 96.0,
                    // A child's body (v27).
                    age_years: Some(10.0),
                }),
                pieces: vec![SourcePiece { panel, piece: 200 }],
            }),
            ..first
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
            a: SeamSide::edge(contour[1], false),
            b: SeamSide {
                edges: vec![contour[2], contour[3]],
                line: None,
                from_mm: Some(20.0),
                to_mm: Some(500.0),
                reversed: true,
            },
            gathered: true,
            ease_mm: 6.0,
        },
    );
    // A hole (clockwise), and a seam along an internal line below.
    let hole_pts: Vec<_> = [(150.0, 300.0), (170.0, 330.0), (190.0, 300.0)]
        .iter()
        .map(|&(x, y)| {
            let id = tx.new_point_id();
            tx.put_point(id, Point::new(panel, DVec2::new(x, y)));
            id
        })
        .collect();
    let hole: Vec<_> = (0..3)
        .map(|i| {
            let id = tx.new_edge_id();
            tx.put_edge(
                id,
                Edge::new(panel, hole_pts[i], hole_pts[(i + 1) % 3], EdgeCurve::Line),
            );
            id
        })
        .collect();
    let mut with_hole = tx.project().panel(panel).unwrap().clone();
    with_hole.holes = vec![hole];
    tx.put_panel(panel, with_hole);
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
    let pocket_seam = tx.new_seam_id();
    tx.put_seam(
        pocket_seam,
        Seam {
            a: SeamSide {
                edges: Vec::new(),
                line: Some(line),
                from_mm: None,
                to_mm: Some(100.0),
                reversed: false,
            },
            b: SeamSide {
                edges: vec![contour[0]],
                line: None,
                from_mm: Some(50.0),
                to_mm: Some(150.0),
                reversed: true,
            },
            gathered: false,
            ease_mm: 0.0,
        },
    );
    tx.set_avatar(Some(Avatar {
        body: Body {
            masculinity: 0.0,
            height_cm: 168.0,
            bust_cm: 90.0,
            waist_cm: 73.0,
            hips_cm: 98.0,
            age_years: None,
        },
        preset: Some("Women M".into()),
        // An imported body (v18): a scan in centimetres, Z up, facing -Z.
        mesh: Some(AvatarMesh {
            file: "avatars/scan.obj".into(),
            hash: "0123456789abcdef".into(),
            scale: 0.01,
            up: UpAxis::Z,
            turn_deg: 180.0,
            points: [
                (BodyPoint::Waist, [0.0, 1050.0, 10.0]),
                (BodyPoint::LeftElbow, [230.0, 1080.0, -20.0]),
            ]
            .into_iter()
            .collect(),
        }),
        // Measurements typed besides the four (v20).
        measures: [
            (r3d_core::Measure::Neck, 36.0),
            (r3d_core::Measure::ArmLength, 58.5),
        ]
        .into_iter()
        .collect(),
    }));
    let size = |name: &str, bust_cm: f64| Size {
        name: name.into(),
        height_cm: 0.0,
        bust_cm,
        waist_cm: bust_cm,
        hips_cm: bust_cm,
        // Each size a year older (v27).
        age_years: bust_cm / 4.0,
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
            // A name and a label placed by hand (v21).
            name: Some("half_bust".into()),
            label: Some([0.0, -30.0]),
        },
        Constraint::Angle {
            a: contour[0],
            b: contour[1],
            deg: 92.0,
            formula: None,
            name: None,
            label: None,
        },
    ] {
        let id = tx.new_constraint_id();
        tx.put_constraint(id, c);
    }
    tx.set_variables([("ease".to_owned(), "5 cm".to_owned())].into());
    // How the photos are taken (v28).
    tx.set_photo(Some(PhotoSetup {
        light: PhotoLight::Environment {
            name: "environments/loft.hdr".into(),
        },
        turn_deg: 45.0,
        brightness: 1.5,
        place_size_m: 12.0,
        camera: PhotoCamera {
            target_mm: [0.0, 900.0, 0.0],
            yaw_deg: 30.0,
            pitch_deg: 5.0,
            distance_mm: 3200.0,
            focal_mm: 50.0,
        },
        size: Some([1080, 1350]),
    }));
    let guide = tx.new_guide_id();
    tx.put_guide(
        guide,
        Guide {
            a: DVec2::new(-50.0, 300.0),
            b: DVec2::new(450.0, 300.0),
        },
    );
    // Constraints on a notch and a construction line (v21).
    for c in [
        Constraint::NotchDistance {
            notch,
            from: EdgeEnd::End,
            mm: 40.0,
            formula: Some("half_bust / 10".into()),
            name: None,
            label: None,
        },
        Constraint::OnGuide {
            point: pts[2],
            guide,
        },
        Constraint::GuideDistance {
            point: pts[1],
            guide,
            mm: 150.0,
            formula: None,
            name: Some("hem_depth".into()),
            label: None,
        },
    ] {
        let id = tx.new_constraint_id();
        tx.put_constraint(id, c);
    }
    let button = tx.new_button_id();
    tx.put_button(
        button,
        Button {
            panel,
            at: DVec2::new(60.0, 500.0),
            diameter_mm: 11.0,
            hole: Buttonhole {
                panel,
                at: DVec2::new(340.0, 500.0),
                angle_deg: 180.0,
            },
            closed: true,
        },
    );
    let zipper = tx.new_zipper_id();
    tx.put_zipper(
        zipper,
        Zipper {
            a: SeamSide {
                edges: vec![contour[3]],
                line: None,
                from_mm: Some(0.0),
                to_mm: Some(180.0),
                reversed: true,
            },
            b: SeamSide::edge(contour[1], false),
            open_mm: 60.0,
        },
    );
    // An elastic on the hem's middle stretch (v22).
    let elastic = tx.new_detail_id();
    tx.put_detail(
        elastic,
        Detail::Elastic(Elastic {
            side: SeamSide {
                edges: vec![contour[0]],
                line: None,
                from_mm: Some(100.0),
                to_mm: Some(300.0),
                reversed: false,
            },
            ratio: 0.6,
            width_mm: 10.0,
            shirring: false,
        }),
    );
    // Two rows of heavy white topstitching along the top (v23).
    let topstitch = tx.new_detail_id();
    tx.put_detail(
        topstitch,
        Detail::Topstitch(Topstitch {
            side: SeamSide::edge(contour[2], false),
            rows: 2,
            offset_mm: 1.5,
            gap_mm: 5.0,
            stitch_mm: 3.0,
            color: Some("#FFFFFF".into()),
            thread: Thread::Heavy,
        }),
    );
    // A heavy non-woven interfacing in a 40 mm band inside the top (v24).
    let interfacing = tx.new_detail_id();
    tx.put_detail(
        interfacing,
        Detail::Interfacing(Interfacing {
            panel,
            area: InterfacingArea::Band {
                edges: vec![contour[2]],
                width_mm: 40.0,
            },
            fabric: InterfacingFabric::NonWoven,
            weight: InterfacingWeight::Heavy,
        }),
    );
    // The piece quilted in 50 mm diamonds over an 8 mm padding (v25).
    let quilting = tx.new_detail_id();
    tx.put_detail(
        quilting,
        Detail::Quilting(Quilting {
            panel,
            line: None,
            pattern: QuiltPattern::Diamonds,
            angle_deg: 45.0,
            spacing_mm: 50.0,
            loft_mm: 8.0,
            padding_gsm: 100.0,
            stitch_mm: 3.0,
            color: None,
        }),
    );
    // A shirring row across the piece (v29).
    let row = tx.new_internal_line_id();
    tx.put_internal_line(
        row,
        InternalLine {
            panel,
            start: DVec2::new(1.0, 540.0),
            segments: vec![PathSegment::Line {
                to: DVec2::new(399.0, 540.0),
            }],
        },
    );
    let shirring = tx.new_detail_id();
    tx.put_detail(
        shirring,
        Detail::Elastic(Elastic {
            side: SeamSide {
                edges: Vec::new(),
                line: Some(row),
                from_mm: None,
                to_mm: None,
                reversed: false,
            },
            ratio: 0.75,
            width_mm: 2.0,
            shirring: true,
        }),
    );
    tx.commit();
    p
}

/// A project that makes an object rather than a garment worn (v30): no
/// avatar, a stuffing.
fn object_sample() -> Project {
    let mut p = Project::new("Golden object");
    let mut tx = Transaction::begin(&mut p);
    tx.set_object(Some(ObjectSetup { stuffing: 0.7 }));
    tx.commit();
    p
}

#[test]
fn current_version_matches_golden() {
    for (name, project) in [("sample.json", sample()), ("object.json", object_sample())] {
        project.check_integrity().unwrap();
        let json = file::to_json(&project).unwrap();
        let path = golden_dir(SCHEMA_VERSION).join(name);
        if std::env::var_os("UPDATE_GOLDEN").is_some() {
            fs::write(&path, &json).unwrap();
        }
        let expected = fs::read_to_string(&path).unwrap();
        assert_eq!(
            json, expected,
            "golden file {name} differs; run with UPDATE_GOLDEN=1 if intended"
        );
    }
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
