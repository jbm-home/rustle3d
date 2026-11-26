#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // test code
//! Every starter garment, at its default sizes on a standard body, is sewn
//! with matched seams and drapes onto the body (docs/specs/11).

mod common;

use r3d_commands::drape::{DEFAULT_SPACING_MM, Form, build};
use r3d_commands::{Session, Step};
use r3d_core::{Around, Project};
use r3d_sim::{CpuSolver, Solver};
use serde_json::{Value, json};

fn run(s: &mut Session, v: Value) -> Value {
    s.run_step(&Step::from_value(v).unwrap()).unwrap()
}

/// Lowest and highest cloth point (m) after `frames` frames.
fn drape(s: &Session, frames: u32) -> (f32, f32) {
    let form = Form::of_project(s.project()).unwrap().unwrap();
    let drape = build(s.project(), Some(&form), DEFAULT_SPACING_MM).unwrap();
    let collider = drape.scene.collider.clone().unwrap();
    let triangles: Vec<[u32; 3]> = drape.scene.triangles.iter().map(|t| t.vertices).collect();
    let mut solver = CpuSolver::new(drape.scene);
    for _ in 0..frames {
        solver.step(1.0 / 60.0).unwrap();
    }
    let p = solver.positions();
    assert!(p.iter().all(|v| v.is_finite()));
    // No cloth inside the body.
    let inside = p
        .iter()
        .filter(|q| collider.sample(**q).is_some_and(|c| c.distance < 0.0))
        .count();
    assert_eq!(inside, 0, "particles inside the body");
    // Nor between particles: triangles are flat, and across a convex part
    // of the body (the bust) their middle could sink in.
    let sunk = triangles
        .iter()
        .map(|t| t.map(|i| p[i as usize]))
        .flat_map(|[a, b, c]| {
            [
                (a + b + c) / 3.0,
                (a + b) / 2.0,
                (b + c) / 2.0,
                (c + a) / 2.0,
            ]
        })
        .filter_map(|q| collider.sample(q).map(|c| -c.distance))
        .fold(0.0f32, f32::max);
    // Within 1 mm: unseen, where 2 mm showed the body through a dress.
    assert!(sunk < 0.001, "triangles cut {sunk} m into the body");
    let stats = solver.stats();
    assert!(stats.max_seam_gap < 0.005, "seams open: {stats:?}");
    assert!(stats.max_closure_gap < 0.005, "unbuttoned: {stats:?}");
    let low = p.iter().map(|v| v.y).fold(f32::MAX, f32::min);
    let high = p.iter().map(|v| v.y).fold(f32::MIN, f32::max);
    (low, high)
}

fn draft(template: &str) -> Session {
    let mut s = Session::new(Project::new("Untitled"));
    common::women_m(&mut s);
    run(
        &mut s,
        json!({"type": "template.apply", "template": template}),
    );
    let seams = run(&mut s, json!({"type": "seam.list"}));
    for seam in seams.as_array().unwrap() {
        assert!(seam["mismatch_mm"].as_f64().unwrap() < 1.0, "{seam}");
    }
    s
}

#[test]
fn the_list_fills_sizes_from_the_avatar() {
    let mut s = Session::new(Project::new("t"));
    let list = run(&mut s, json!({"type": "template.list"}));
    let shirt = &list[0];
    assert_eq!(shirt["template"], "t_shirt");
    // Without an avatar: the standard size.
    assert_eq!(shirt["sizes"]["chest_cm"], 90.0);
    assert!(shirt["sizes"].get("waist_cm").is_none());
    run(
        &mut s,
        json!({"type": "avatar.set", "body": {"masculinity": 1, "height_cm": 182,
            "bust_cm": 104, "waist_cm": 90, "hips_cm": 102}}),
    );
    let list = run(&mut s, json!({"type": "template.list"}));
    assert_eq!(list[0]["sizes"]["chest_cm"], 104.0);
    assert_eq!(list[1]["template"], "shirt");
    assert_eq!(list[2]["template"], "trousers");
    assert_eq!(list[2]["sizes"]["waist_cm"], 90.0);
    assert!(list[2]["sizes"].get("sleeve_cm").is_none());
}

#[test]
fn a_t_shirt_hangs_from_the_shoulders() {
    let s = draft("t_shirt");
    let p = s.project();
    // The garment takes the template's name; the project keeps its own.
    assert_eq!(p.garments().values().next().unwrap().name, "T-shirt");
    assert_eq!(p.panels().len(), 4);
    assert_eq!(p.seams().len(), 10);
    let on_arms = p
        .panels()
        .values()
        .filter(|pn| pn.placement.is_some_and(|pl| pl.around != Around::Body))
        .count();
    assert_eq!(on_arms, 2);
    let (low, high) = drape(&s, 180);
    eprintln!("t-shirt: {low:.3}..{high:.3}");
    // A 168 cm body: the neckline stays near the base of the neck (1.39 m
    // to the shoulder tops), the hem above the knees.
    assert!(high > 1.35 && high < 1.50, "top at {high}");
    assert!(low > 0.7, "hem at {low}");
}

#[test]
fn a_t_shirt_comes_to_rest() {
    // Edges held out of the body used to let go of the particles they had
    // pushed off the skin; they fell back and were pushed again every
    // substep, and the shirt kept fluttering (12 cm/s on average, for
    // ever). Settled cloth moves about 1 mm/s.
    let s = draft("t_shirt");
    let form = Form::of_project(s.project()).unwrap().unwrap();
    let drape = build(s.project(), Some(&form), DEFAULT_SPACING_MM).unwrap();
    let mut solver = CpuSolver::new(drape.scene);
    for _ in 0..240 {
        solver.step(1.0 / 60.0).unwrap();
    }
    let before = solver.positions().to_vec();
    solver.step(1.0 / 60.0).unwrap();
    let speed = solver
        .positions()
        .iter()
        .zip(&before)
        .map(|(a, b)| a.distance(*b) * 60.0)
        .sum::<f32>()
        / before.len() as f32;
    assert!(speed < 0.01, "still moving at {speed} m/s on average");
}

#[test]
fn long_sleeves_follow_the_forearms() {
    // Past the elbow a sleeve starts around the forearm, not along the
    // upper arm's axis carried on (which left it off the arm, crumpled,
    // its seams open).
    let mut s = Session::new(Project::new("Untitled"));
    common::women_m(&mut s);
    run(
        &mut s,
        json!({"type": "template.apply", "template": "t_shirt",
            "sizes": {"sleeve_cm": 50}}),
    );
    let (low, high) = drape(&s, 300);
    eprintln!("long sleeves: {low:.3}..{high:.3}");
    assert!(high > 1.35 && high < 1.50, "top at {high}");
}

#[test]
fn a_shirt_has_a_collar_and_cuffs() {
    let s = draft("shirt");
    let p = s.project();
    // The garment takes the template's name; the project keeps its own.
    assert_eq!(p.garments().values().next().unwrap().name, "Shirt");
    // Two fronts, a yoke and a back, two sleeves, two cuffs, a collar;
    // the fronts and the cuffs buttoned. Each sleeve's back cap is sewn
    // to the back and to the yoke.
    assert_eq!(p.panels().len(), 9);
    assert_eq!(p.seams().len(), 18);
    // The collar and the cuffs interfaced, the yoke topstitched.
    assert_eq!(p.interfacings().count(), 3);
    assert_eq!(p.topstitches().count(), 1);
    assert_eq!(p.buttons().len(), 9);
    assert!(p.buttons().values().all(|b| b.closed));
    let (low, high) = drape(&s, 180);
    eprintln!("shirt: {low:.3}..{high:.3}");
    // The collar stands at the base of the neck; the hem below the hips.
    assert!(high > 1.42 && high < 1.52, "top at {high}");
    assert!(low > 0.65 && low < 0.85, "hem at {low}");
}

#[test]
fn trousers_hang_from_the_waist_one_leg_in_each_leg() {
    // Trousers wrapped around the body started with each inseam sewn
    // across the gap between the legs; around the legs, each leg of the
    // trousers starts and stays on its own side.
    let s = draft("trousers");
    let p = s.project();
    // The garment takes the template's name; the project keeps its own.
    assert_eq!(p.garments().values().next().unwrap().name, "Trousers");
    assert_eq!(p.panels().len(), 4);
    assert_eq!(p.seams().len(), 6);
    // A fly zipper, closed, above the crotch seam.
    assert_eq!(p.zippers().len(), 1);
    let fly = p.zippers().values().next().unwrap();
    assert!((p.zipper_length(fly).unwrap() - 180.0).abs() < 1.0);
    assert_eq!(p.zipper_closed(fly), 1.0);
    let form = Form::of_project(p).unwrap().unwrap();
    let built = build(p, Some(&form), DEFAULT_SPACING_MM).unwrap();
    let mut solver = CpuSolver::new(built.scene.clone());
    for _ in 0..180 {
        solver.step(1.0 / 60.0).unwrap();
    }
    let positions = solver.positions();
    for piece in &built.pieces {
        let panel = p.panel(piece.panel).unwrap();
        let side = if panel.name.ends_with("left") {
            1.0
        } else {
            -1.0
        };
        let below_crotch = positions[piece.particles.clone()]
            .iter()
            .filter(|q| q.y < 0.7);
        for q in below_crotch {
            assert!(side * q.x > 0.0, "{} crosses over at {q}", panel.name);
        }
    }
    let (low, high) = drape(&s, 180);
    eprintln!("trousers: {low:.3}..{high:.3}");
    // The waist stays at the waist (1.04 m), the hem just above the floor
    // (the template's length is the waist's height less 4 cm: about 2 cm
    // once draped).
    assert!(high > 1.0, "trousers slid down: top at {high}");
    assert!(low > 0.01 && low < 0.05, "hem at {low}");
}

/// The standard men's L body, with `template` drafted on it.
fn mens(template: &str) -> Session {
    let mut s = Session::new(Project::new("Untitled"));
    run(
        &mut s,
        json!({"type": "avatar.set", "body": {"masculinity": 1, "height_cm": 182,
            "bust_cm": 106, "waist_cm": 94, "hips_cm": 104}}),
    );
    run(
        &mut s,
        json!({"type": "template.apply", "template": template}),
    );
    s
}

#[test]
fn trousers_fit_a_mens_body() {
    let (low, high) = drape(&mens("trousers"), 180);
    eprintln!("men's trousers: {low:.3}..{high:.3}");
    // They settle a few centimetres under the waist (1.08 m), held by
    // friction alone: the waist's hold is open (ROADMAP, open points).
    assert!(high > 1.03, "trousers slid down: top at {high}");
    assert!(low > 0.01 && low < 0.05, "hem at {low}");
}

#[test]
fn a_shirt_fits_a_mens_body() {
    // A men's sleeve started open at the elbow (outside the arm's reach)
    // and its seams caught a fold closing it.
    let (low, high) = drape(&mens("shirt"), 180);
    eprintln!("men's shirt: {low:.3}..{high:.3}");
    assert!(high > 1.5 && high < 1.65, "top at {high}");
}

#[test]
fn a_dress_hangs_from_the_shoulders() {
    let s = draft("dress");
    assert_eq!(s.project().panels().len(), 2);
    assert_eq!(s.project().seams().len(), 6);
    let (low, high) = drape(&s, 180);
    eprintln!("dress: {low:.3}..{high:.3}");
    assert!(high > 1.38 && high < 1.50, "top at {high}");
    assert!(low > 0.45 && low < 0.7, "hem at {low}");
}

#[test]
fn a_tote_bag_hangs_from_its_handles() {
    let s = draft("tote_bag");
    let p = s.project();
    assert_eq!(p.panels().len(), 2);
    assert_eq!(p.seams().len(), 5);
    assert_eq!(p.pins().len(), 2);
    let (low, high) = drape(&s, 180);
    eprintln!("tote: {low:.3}..{high:.3}");
    // Held at 1.25 m by the pins; 28 cm handles above a 40 cm bag.
    assert!((high - 1.25).abs() < 0.02, "top at {high}");
    assert!(low > 0.5 && low < 0.65, "bottom at {low}");
}

#[test]
fn skirts_hang_from_the_waist() {
    for template in ["straight_skirt", "a_line_skirt"] {
        let s = draft(template);
        assert_eq!(s.project().panels().len(), 3);
        let (low, high) = drape(&s, 120);
        eprintln!("{template}: {low:.3}..{high:.3}");
        assert!(high > 1.03, "{template} slid down: top at {high}");
        assert!(low > 0.3, "{template} hem at {low}");
    }
}

#[test]
fn a_hoodie_has_a_hood_and_gathered_cuffs() {
    let s = draft("hoodie");
    let p = s.project();
    assert_eq!(p.garments().values().next().unwrap().name, "Hoodie");
    // The T-shirt's four pieces and two hood halves; the hood sewn down
    // its middle and to both necklines.
    assert_eq!(p.panels().len(), 6);
    assert_eq!(p.seams().len(), 15);
    // Elastic at both cuffs and along the hem.
    assert_eq!(p.elastics().count(), 4);
    let (low, high) = drape(&s, 180);
    eprintln!("hoodie: {low:.3}..{high:.3}");
    // The hood settles over the head and falls down the back, below the
    // head's top (1.68 m); the hem gathered above the thighs.
    assert!(high > 1.42 && high < 1.65, "top at {high}");
    assert!(low > 0.7, "hem at {low}");
}

#[test]
fn joggers_hang_from_their_elastic_waist() {
    let s = draft("joggers");
    let p = s.project();
    assert_eq!(p.garments().values().next().unwrap().name, "Joggers");
    assert_eq!(p.panels().len(), 4);
    // Pulled on: no fly, the centre front sewn all the way.
    assert_eq!(p.seams().len(), 6);
    assert!(p.zippers().is_empty());
    // An elastic along each piece's waist and hem.
    assert_eq!(p.elastics().count(), 8);
    let (low, high) = drape(&s, 180);
    eprintln!("joggers: {low:.3}..{high:.3}");
    // The elastic holds the waist (1.04 m) up; the ankles gathered just
    // above the floor.
    assert!(high > 1.0, "joggers slid down: top at {high}");
    assert!(low > 0.02 && low < 0.1, "hem at {low}");
}

#[test]
fn jeans_have_five_pockets_a_yoke_and_a_waistband() {
    let s = draft("jeans");
    let p = s.project();
    assert_eq!(p.garments().values().next().unwrap().name, "Jeans");
    let named = |name: &str| {
        p.panels()
            .values()
            .filter(|pn| pn.name.contains(name))
            .count()
    };
    assert_eq!(named("Front"), 2);
    assert_eq!(named("Pocket facing"), 2);
    assert_eq!(named("Back pocket"), 2);
    assert_eq!(named("Coin pocket"), 1);
    assert_eq!(named("Yoke"), 2);
    assert_eq!(named("Waistband"), 2);
    assert_eq!(named("Belt loop"), 6);
    // The fly.
    assert_eq!(p.zippers().len(), 1);
    let (low, high) = drape(&s, 180);
    eprintln!("jeans: {low:.3}..{high:.3}");
    // The waistband over the waist (1.04 m); the hems a few centimetres
    // above the floor, held up by it.
    assert!(high > 1.06, "jeans slid down: top at {high}");
    assert!(low > 0.02 && low < 0.09, "hem at {low}");
}

#[test]
fn shorts_end_above_the_knees() {
    let s = draft("shorts");
    let p = s.project();
    assert_eq!(p.garments().values().next().unwrap().name, "Shorts");
    assert_eq!(p.panels().len(), 4);
    assert_eq!(p.zippers().len(), 1);
    let (low, high) = drape(&s, 180);
    eprintln!("shorts: {low:.3}..{high:.3}");
    // The waist at the waist (1.04 m), the hems a hand above the knees
    // (about 0.48 m on this body).
    assert!(high > 1.0, "shorts slid down: top at {high}");
    assert!(low > 0.45 && low < 0.62, "hem at {low}");
}

/// A child's body of `masculinity`, `age` years, from its chart
/// measurements, with `template` drafted on it.
fn child(masculinity: f64, body: [f64; 5], template: &str) -> Session {
    let [age, height, chest, waist, hips] = body;
    let mut s = Session::new(Project::new("Untitled"));
    run(
        &mut s,
        json!({"type": "avatar.set", "body": {"masculinity": masculinity,
            "height_cm": height, "bust_cm": chest, "waist_cm": waist,
            "hips_cm": hips, "age_years": age}}),
    );
    run(
        &mut s,
        json!({"type": "template.apply", "template": template}),
    );
    let seams = run(&mut s, json!({"type": "seam.list"}));
    for seam in seams.as_array().unwrap() {
        assert!(seam["mismatch_mm"].as_f64().unwrap() < 1.0, "{seam}");
    }
    s
}

#[test]
fn a_babys_bodysuit_closes_under_the_crotch() {
    // A 24 months baby, 88 cm. (At 12 months the arm rests against the
    // chest: the cloth cuts 2 cm into the armpit's crease between its
    // vertices, spec 33.)
    let s = child(0.5, [2.0, 88.0, 51.0, 49.0, 52.0], "bodysuit");
    let p = s.project();
    assert_eq!(p.garments().values().next().unwrap().name, "Bodysuit");
    // The T-shirt's pieces and seams, and the crotch.
    assert_eq!(p.panels().len(), 4);
    assert_eq!(p.seams().len(), 11);
    let (low, high) = drape(&s, 180);
    eprintln!("bodysuit: {low:.3}..{high:.3}");
    // From the shoulders (about 0.7 m) down to under the crotch (0.34
    // m), the legs free.
    assert!(high > 0.65 && high < 0.8, "top at {high}");
    assert!(low > 0.25 && low < 0.38, "crotch at {low}");
}

#[test]
fn a_babys_bodysuit_takes_a_babys_eases() {
    // A 12 months baby, 76 cm, 47 cm chest, 14.8 cm round the upper arm.
    // An adult's eases (7 cm under the arm, 6 cm across the sleeve) made
    // its sleeves 21.5 cm wide and its armholes 15 cm deep.
    let s = child(0.5, [1.0, 76.0, 47.0, 46.0, 48.0], "bodysuit");
    let p = s.project();
    let width = |name: &str| {
        let (id, _) = p.panels().iter().find(|(_, pn)| pn.name == name).unwrap();
        let b = p.panel_bounding_box(*id).unwrap();
        (b.max.x - b.min.x, b.max.y - b.min.y)
    };
    let (sleeve, _) = width("Sleeve left");
    assert!(sleeve > 165.0 && sleeve < 195.0, "sleeve {sleeve} mm wide");
    let (front, _) = width("Front");
    assert!(front < 280.0, "front {front} mm wide");
}

#[test]
fn a_pyjama_is_its_bottoms_and_a_top_worn_over_them() {
    // A 4-year-old girl, 104 cm.
    let s = child(0.0, [4.0, 104.0, 56.0, 53.0, 59.0], "pyjama");
    let p = s.project();
    let garments: Vec<_> = p.garments().values().collect();
    let names: Vec<&str> = garments.iter().map(|g| g.name.as_str()).collect();
    assert_eq!(names, ["Pyjama bottoms", "Pyjama top"]);
    assert!(garments[1].layer > garments[0].layer);
    for g in &garments {
        assert!(g.template.is_some(), "{} keeps its template", g.name);
    }
    assert_eq!(p.panels().len(), 8);
    // An elastic at each piece's waist and at both cuffs.
    assert_eq!(p.elastics().count(), 6);
    let (low, high) = drape(&s, 180);
    eprintln!("pyjama: {low:.3}..{high:.3}");
    assert!(high > 0.8 && high < 0.95, "top at {high}");
    // The legs open at the ankles, down on the feet.
    assert!(low < 0.08, "hems at {low}");
}

#[test]
fn a_jogging_suit_is_joggers_and_a_sweatshirt() {
    let s = child(1.0, [8.0, 128.0, 66.0, 58.0, 67.0], "jogging_suit");
    let names: Vec<String> = s
        .project()
        .garments()
        .values()
        .map(|g| g.name.clone())
        .collect();
    assert_eq!(names, ["Joggers", "Sweatshirt"]);
    // Joggers' 8 elastics, the sweatshirt's cuffs and hem.
    assert_eq!(s.project().elastics().count(), 12);
}

#[test]
fn a_childs_range_is_graded_by_age() {
    let mut s = child(0.0, [4.0, 104.0, 56.0, 53.0, 59.0], "pyjama_top");
    run(
        &mut s,
        json!({"type": "grading.use_chart", "chart": "girls", "from": "2", "to": "6", "base": "4"}),
    );
    let p = s.project();
    let ages: Vec<Option<f64>> = (0..5).map(|i| p.size_body(i).unwrap().age_years).collect();
    assert_eq!(
        ages,
        [Some(2.0), Some(3.0), Some(4.0), Some(5.0), Some(6.0)]
    );
    // The top drafted on each size's body: every outline point graded.
    assert!(p.points().values().all(|pt| pt.grade.len() == 5));
    // A child is 0 to 18 years old.
    let too_old = Step::from_value(json!({"type": "avatar.set", "body": {"masculinity": 0,
        "height_cm": 160, "bust_cm": 80, "waist_cm": 65, "hips_cm": 88, "age_years": 20}}))
    .unwrap();
    assert!(s.run_step(&too_old).is_err());
}
