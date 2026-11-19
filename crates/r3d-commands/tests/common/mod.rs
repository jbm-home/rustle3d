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

/// The particles of garment `inner` (as numbered in the scene's
/// triangles) that show past garment `outer` from the front or the back, between
/// heights `low` and `high` (m) and within 10 cm of the median plane: in
/// front of the outer cloth's frontmost surface at their height and
/// across, or behind its backmost one, by more than a millimetre.
pub fn seen_through(
    scene: &r3d_sim::SimScene,
    x: &[glam::Vec3],
    inner: u16,
    outer: u16,
    low: f32,
    high: f32,
) -> Vec<usize> {
    let mut garment = vec![u16::MAX; x.len()];
    for t in &scene.triangles {
        for v in t.vertices {
            garment[v as usize] = t.garment;
        }
    }
    // Where the outer cloth is along z at (px, py), on the side of `front`.
    let surface = |px: f32, py: f32, front: bool| {
        let mut best: Option<f32> = None;
        for t in scene.triangles.iter().filter(|t| t.garment == outer) {
            let [a, b, c] = t.vertices.map(|v| x[v as usize]);
            if (a.z > 0.0) != front {
                continue;
            }
            let (e1, e2) = (b - a, c - a);
            let den = e1.x * e2.y - e2.x * e1.y;
            if den.abs() < 1e-12 {
                continue;
            }
            let (qx, qy) = (px - a.x, py - a.y);
            let s = (qx * e2.y - e2.x * qy) / den;
            let r = (e1.x * qy - qx * e1.y) / den;
            if s < 0.0 || r < 0.0 || s + r > 1.0 {
                continue;
            }
            let z = a.z + s * e1.z + r * e2.z;
            best = Some(best.map_or(z, |m: f32| if front { m.max(z) } else { m.min(z) }));
        }
        best
    };
    x.iter()
        .zip(&garment)
        .enumerate()
        .filter(|(_, (p, g))| **g == inner && (low..high).contains(&p.y) && p.x.abs() < 0.1)
        .filter(|(_, (p, _))| {
            let front = p.z > 0.0;
            surface(p.x, p.y, front).is_some_and(|z| {
                if front {
                    p.z > z + 0.001
                } else {
                    p.z < z - 0.001
                }
            })
        })
        .map(|(i, _)| i)
        .collect()
}
