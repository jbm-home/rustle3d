#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // test code

use r3d_core::{Edge, EdgeCurve, Notch, Panel, Point, Project, Transaction};
use r3d_geom::DVec2;
use r3d_io::drawing::{Stroke, TEST_SQUARE_MM, pattern_drawing};
use r3d_io::pdf::{Paper, layout, to_pdf};
use r3d_io::svg::to_svg;

/// One rectangular panel `w × h` with a seam allowance and a notch.
fn project(w: f64, h: f64, allowance: f64) -> Project {
    let mut p = Project::new("Test");
    let mut tx = Transaction::begin(&mut p);
    let garment = tx.project().default_garment().unwrap();
    let panel = tx.new_panel_id();
    let corners = [(0.0, 0.0), (w, 0.0), (w, h), (0.0, h)];
    let pts: Vec<_> = corners
        .iter()
        .map(|&(x, y)| {
            let id = tx.new_point_id();
            tx.put_point(id, Point::new(panel, DVec2::new(x, y)));
            id
        })
        .collect();
    let contour: Vec<_> = (0..4)
        .map(|i| {
            let id = tx.new_edge_id();
            tx.put_edge(
                id,
                Edge::new(panel, pts[i], pts[(i + 1) % 4], EdgeCurve::Line),
            );
            id
        })
        .collect();
    let notch = tx.new_notch_id();
    tx.put_notch(
        notch,
        Notch {
            edge: contour[0],
            offset_mm: w / 2.0,
        },
    );
    tx.put_panel(
        panel,
        Panel {
            name: "Front & <back>".into(),
            garment,
            contour,
            grain_angle_deg: 90.0,
            seam_allowance_mm: allowance,
            mirror_of: None,
            placement: None,
            symmetry: None,
            holes: Vec::new(),
        },
    );
    tx.commit();
    p
}

#[test]
fn drawing_has_cut_sew_notch_grain_and_test_square() {
    let d = pattern_drawing(&project(400.0, 600.0, 10.0));
    let count = |s| d.lines.iter().filter(|l| l.stroke == s).count();
    assert_eq!(count(Stroke::Cut), 1);
    assert_eq!(count(Stroke::Sew), 1);
    assert_eq!(count(Stroke::Notch), 1);
    assert!(count(Stroke::Grain) >= 1);
    let square = d.lines.iter().find(|l| l.stroke == Stroke::Mark).unwrap();
    assert!((square.points[1].x - square.points[0].x - TEST_SQUARE_MM).abs() < 1e-9);
    // The cutting line is the sewing line grown by the allowance.
    let cut = d.lines.iter().find(|l| l.stroke == Stroke::Cut).unwrap();
    let area = r3d_geom::polygon::signed_area(&cut.points);
    assert!((area - 420.0 * 620.0).abs() < 1e-6);
    // The notch is cut inward from the cutting line, at mid-width.
    let notch = d.lines.iter().find(|l| l.stroke == Stroke::Notch).unwrap();
    assert_eq!(notch.points[0], DVec2::new(200.0, -10.0));
}

#[test]
fn svg_is_one_to_one_and_escapes_text() {
    let d = pattern_drawing(&project(400.0, 600.0, 0.0));
    let svg = to_svg(&d);
    let size = d.bounds.max - d.bounds.min;
    assert!(svg.contains(&format!(r#"width="{:.3}mm""#, size.x)));
    assert!(svg.contains(&format!(r#"viewBox="0 0 {:.3} {:.3}""#, size.x, size.y)));
    assert!(svg.contains("Front &amp; &lt;back&gt;"));
    assert!(svg.contains(">100 mm<"));
}

#[test]
fn tiles_cover_the_drawing_with_overlap() {
    let d = pattern_drawing(&project(1000.0, 700.0, 10.0));
    let (page, window, tiles) = layout(&d, Paper::A4);
    assert_eq!(page, DVec2::new(210.0, 297.0));
    let size = d.bounds.max - d.bounds.min;
    let right = tiles
        .iter()
        .map(|t| t.origin.x + window.x)
        .fold(f64::MIN, f64::max);
    let bottom = tiles.iter().map(|t| t.origin.y).fold(f64::MAX, f64::min);
    assert!(
        right >= d.bounds.max.x && bottom <= d.bounds.min.y,
        "tiles do not cover"
    );
    assert!(tiles.len() >= ((size.x / window.x).ceil() * (size.y / window.y).ceil()) as usize);
    assert_eq!(tiles[0].label, "A1");
    // Top-left tile contains the test square (placed above the panels).
    assert!(tiles[0].origin.y + window.y >= d.bounds.max.y);

    let (_, _, single) = layout(&d, Paper::Single);
    assert_eq!(single.len(), 1);
}

#[test]
fn pdf_has_one_page_per_tile() {
    let d = pattern_drawing(&project(1000.0, 700.0, 10.0));
    let (_, _, tiles) = layout(&d, Paper::A4);
    let bytes = to_pdf(&d, Paper::A4);
    assert!(bytes.starts_with(b"%PDF-"));
    let text = String::from_utf8_lossy(&bytes);
    assert_eq!(
        text.matches("/Type /Page\n").count() + text.matches("/Type /Page ").count(),
        tiles.len()
    );
    // A4 = 210 × 297 mm = 595.28 × 841.89 pt.
    let media: Vec<f64> = text
        .split("/MediaBox [")
        .nth(1)
        .and_then(|rest| rest.split(']').next())
        .unwrap()
        .split_whitespace()
        .map(|n| n.parse().unwrap())
        .collect();
    assert!(
        (media[2] - 595.28).abs() < 0.01 && (media[3] - 841.89).abs() < 0.01,
        "{media:?}"
    );
}
