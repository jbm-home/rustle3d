use r3d_geom::DVec2;

use super::*;
use crate::import::grain_angle_deg;

/// A quarter skirt drafted from the waist and the hips (cm), with a
/// curved side, a notch at the hip, a hip line inside, and a second piece
/// cut from the side curve; one more piece uses a step not supported.
const SKIRT: &str = r##"<?xml version="1.0" encoding="UTF-8"?>
<pattern>
  <version>1.2.1</version>
  <unit>cm</unit>
  <measurements path="body.vit"/>
  <increments>
    <increment name="#ease" formula="1" description=""/>
    <increment name="#sep" type="separator"/>
  </increments>
  <draw name="Skirt">
    <calculation>
      <point type="single" id="1" name="A" x="0" y="0" mx="0" my="0"/>
      <point type="endLine" id="2" name="A1" basePoint="1" length="waist_circ/4 + #ease" angle="0" typeLine="hair"/>
      <point type="endLine" id="3" name="A2" basePoint="1" length="60" angle="270" typeLine="hair"/>
      <point type="endLine" id="4" name="A3" basePoint="3" length="hip_circ/4 + #ease" angle="0" typeLine="hair"/>
      <point type="alongLine" id="5" name="A4" firstPoint="1" secondPoint="3" length="CurrentLength/3" typeLine="none"/>
      <point type="endLine" id="6" name="A5" basePoint="5" length="Line_A2_A3" angle="0" typeLine="hair"/>
      <line id="20" firstPoint="6" secondPoint="4" typeLine="hair"/>
      <spline type="simpleInteractive" id="7" point1="2" point4="6" angle1="270" length1="5" angle2="90" length2="5"/>
      <point type="cutSpline" id="8" name="A6" spline="7" length="Spl_A1_A5/2" segment1Id="9" segment2Id="10"/>
      <spline type="parallelCurve" id="11" curve="7" width="1" suffix="p"/>
    </calculation>
    <modeling>
      <point type="modeling" id="100" idObject="1"/>
      <point type="modeling" id="101" idObject="2"/>
      <spline type="modelingSpline" id="102" idObject="7"/>
      <point type="modeling" id="103" idObject="6"/>
      <point type="modeling" id="104" idObject="4"/>
      <point type="modeling" id="105" idObject="3"/>
      <point type="modeling" id="106" idObject="8"/>
      <point type="modeling" id="108" idObject="5"/>
      <spline type="modelingSpline" id="109" idObject="9"/>
      <spline type="modelingPath" id="110" idObject="11"/>
      <path type="2" id="107" name="Hip line" visible="1">
        <nodes>
          <node type="NodePoint" idObject="108"/>
          <node type="NodePoint" idObject="103"/>
        </nodes>
      </path>
    </modeling>
    <details>
      <detail id="200" version="2" name="Front" seamAllowance="true" width="1" mx="0" my="0">
        <data letter="" visible="true" mx="0" my="0" width="1" height="1" rotation="0"/>
        <grainline enabled="true" mx="0" my="0" length="10" rotation="90" arrows="0"/>
        <nodes>
          <node type="NodePoint" idObject="100"/>
          <node type="NodePoint" idObject="101"/>
          <node type="NodeSpline" idObject="102" reverse="0"/>
          <node type="NodePoint" idObject="103" passmark="true" passmarkLine="one"/>
          <node type="NodePoint" idObject="104"/>
          <node type="NodePoint" idObject="105"/>
        </nodes>
        <iPaths><record path="107"/></iPaths>
      </detail>
      <detail id="201" version="2" name="Yoke" seamAllowance="false">
        <nodes>
          <node type="NodePoint" idObject="100"/>
          <node type="NodePoint" idObject="101"/>
          <node type="NodeSpline" idObject="109" reverse="0"/>
          <node type="NodePoint" idObject="106"/>
        </nodes>
      </detail>
      <detail id="202" version="2" name="Facing">
        <nodes>
          <node type="NodePoint" idObject="100"/>
          <node type="NodeSplinePath" idObject="110"/>
        </nodes>
      </detail>
    </details>
  </draw>
</pattern>"##;

fn body(waist_mm: f64, hips_mm: f64) -> Values {
    [("waist_circ", waist_mm), ("hip_circ", hips_mm)]
        .into_iter()
        .map(|(n, v)| {
            (
                n.to_owned(),
                Value {
                    value: v,
                    length: true,
                },
            )
        })
        .collect()
}

fn vertices(p: &ImportedPiece) -> Vec<DVec2> {
    let mut v = vec![p.outline.start];
    v.extend(p.outline.segments.iter().map(r3d_core::PathSegment::end));
    v
}

fn has(points: &[DVec2], p: DVec2) -> bool {
    points.iter().any(|q| q.distance(p) < 1e-6)
}

#[test]
fn a_skirt_is_drafted_for_the_body() {
    let e = evaluate(SKIRT.as_bytes(), &body(700.0, 960.0)).unwrap();
    assert_eq!(e.unsupported, ["spline:parallelCurve"]);
    assert_eq!(e.ignored, 1, "the facing uses the unsupported curve");
    let ids: Vec<u32> = e.pieces.iter().map(|(id, _)| *id).collect();
    assert_eq!(ids, [200, 201]);
    let front = &e.pieces[0].1;
    assert_eq!(front.name.as_deref(), Some("Front"));
    let v = vertices(front);
    // Waist quarter 175 + 10 mm ease, hips quarter 240 + 10, 600 long.
    for p in [
        DVec2::ZERO,
        DVec2::new(185.0, 0.0),
        DVec2::new(250.0, -200.0),
        DVec2::new(250.0, -600.0),
        DVec2::new(0.0, -600.0),
    ] {
        assert!(has(&v, p), "{p} not in {v:?}");
    }
    assert_eq!(v.first(), v.last());
    assert!(
        front
            .outline
            .segments
            .iter()
            .any(|s| matches!(s, r3d_core::PathSegment::Cubic { .. })),
        "the side stays a curve"
    );
    assert_eq!(front.seam_allowance_mm, 10.0);
    assert_eq!(front.notches.len(), 1);
    assert!(front.notches[0].distance(DVec2::new(250.0, -200.0)) < 1e-9);
    assert_eq!(grain_angle_deg(front.grain.unwrap()), 90.0);
    assert_eq!(front.internal_lines.len(), 1);
    let hip = &front.internal_lines[0];
    assert!(hip.start.distance(DVec2::new(0.0, -200.0)) < 1e-6);
    assert!(hip.end().distance(DVec2::new(250.0, -200.0)) < 1e-6);
}

#[test]
fn a_piece_uses_part_of_a_cut_curve() {
    let e = evaluate(SKIRT.as_bytes(), &body(700.0, 960.0)).unwrap();
    let yoke = &e.pieces[1].1;
    let curve_len: f64 = {
        let mut at = yoke.outline.start;
        let mut total = 0.0;
        for s in &yoke.outline.segments {
            if let r3d_core::PathSegment::Cubic { c1, c2, to } = s {
                total += r3d_geom::CubicBez::new(at, *c1, *c2, *to).arc_length(0.001);
            }
            at = s.end();
        }
        total
    };
    let full = r3d_geom::CubicBez::new(
        DVec2::new(185.0, 0.0),
        DVec2::new(185.0, -50.0),
        DVec2::new(250.0, -150.0),
        DVec2::new(250.0, -200.0),
    )
    .arc_length(0.001);
    assert!(
        (curve_len - full / 2.0).abs() < 0.05,
        "{curve_len} vs {full}"
    );
}

#[test]
fn another_body_moves_the_points() {
    let e = evaluate(SKIRT.as_bytes(), &body(800.0, 1000.0)).unwrap();
    let v = vertices(&e.pieces[0].1);
    assert!(has(&v, DVec2::new(210.0, 0.0)));
    assert!(has(&v, DVec2::new(260.0, -600.0)));
}

#[test]
fn a_missing_measurement_is_named() {
    assert_eq!(
        evaluate(SKIRT.as_bytes(), &Values::new()),
        Err(ValError::UnknownName {
            name: "waist_circ".into()
        })
    );
    assert_eq!(
        linked_measurements(SKIRT.as_bytes()).unwrap().as_deref(),
        Some("body.vit")
    );
}

#[test]
fn pieces_are_laid_out_side_by_side() {
    let r = read(SKIRT.as_bytes(), &body(700.0, 960.0)).unwrap();
    assert_eq!(r.ids, [200, 201]);
    assert_eq!(r.pattern.pieces.len(), 2);
    let right_of_front = r.pattern.pieces[0]
        .outline
        .polyline()
        .iter()
        .map(|p| p.x)
        .fold(f64::NEG_INFINITY, f64::max);
    let left_of_yoke = r.pattern.pieces[1]
        .outline
        .polyline()
        .iter()
        .map(|p| p.x)
        .fold(f64::INFINITY, f64::min);
    assert!(left_of_yoke > right_of_front);
}

/// Seamly2D's dialect: draft blocks, variables, pieces; a bodice half
/// with arcs, a dart trued, a mirrored point and a fold line.
const SEAMLY: &str = r##"<?xml version="1.0" encoding="UTF-8"?>
<pattern>
  <version>0.7.6</version>
  <unit>mm</unit>
  <measurements></measurements>
  <variables>
    <variable name="#w" formula="100" description=""/>
  </variables>
  <draftBlock name="Block 1">
    <calculation>
      <point type="single" id="1" name="O" x="0" y="0"/>
      <point type="endLine" id="2" name="B" basePoint="1" length="#w" angle="0" lineType="solidLine"/>
      <point type="endLine" id="3" name="C" basePoint="1" length="#w" angle="90" lineType="solidLine"/>
      <arc type="simple" id="4" center="1" radius="#w" angle1="0" angle2="90"/>
      <point type="cutArc" id="5" name="D" arc="4" length="Arc_O_4/2"/>
      <point type="pointOfIntersectionCircles" id="8" name="E" c1Center="2" c2Center="3" c1Radius="#w" c2Radius="#w" crossPoint="1"/>
      <operation type="flippingByAxis" id="9" center="1" axisType="1" suffix="m">
        <source><item idObject="2"/></source>
        <destination><item idObject="10" mx="0" my="0"/></destination>
      </operation>
      <point type="lineIntersect" id="11" name="F" p1Line1="1" p2Line1="8" p1Line2="2" p2Line2="3"/>
    </calculation>
    <modeling>
      <point type="modeling" id="20" idObject="1" inUse="true"/>
      <point type="modeling" id="21" idObject="2" inUse="true"/>
      <arc type="modeling" id="22" idObject="6" inUse="true"/>
      <point type="modeling" id="23" idObject="5" inUse="true"/>
      <arc type="modeling" id="24" idObject="7" inUse="true"/>
      <point type="modeling" id="25" idObject="3" inUse="true"/>
      <point type="modeling" id="26" idObject="10" inUse="true"/>
    </modeling>
    <pieces>
      <piece id="30" version="2" name="Quarter" seamAllowance="true" width="10" forbidFlipping="false" inLayout="true">
        <grainline visible="true" mx="0" my="0" length="50" rotation="0" arrows="0"/>
        <nodes>
          <node type="NodePoint" idObject="20"/>
          <node type="NodePoint" idObject="21"/>
          <node type="NodeArc" idObject="22"/>
          <node type="NodePoint" idObject="23" notch="true" notchType="slit"/>
          <node type="NodeArc" idObject="24"/>
          <node type="NodePoint" idObject="25"/>
        </nodes>
      </piece>
      <piece id="31" version="2" name="Wedge" seamAllowance="false">
        <nodes>
          <node type="NodePoint" idObject="26"/>
          <node type="NodePoint" idObject="20"/>
          <node type="NodePoint" idObject="25"/>
        </nodes>
      </piece>
    </pieces>
  </draftBlock>
</pattern>"##;

#[test]
fn seamly_dialect_arcs_and_operations() {
    let e = evaluate(SEAMLY.as_bytes(), &Values::new()).unwrap();
    assert!(e.unsupported.is_empty(), "{:?}", e.unsupported);
    assert_eq!(e.ignored, 0);
    let quarter = &e.pieces[0].1;
    // A quarter circle of radius 100 cut in two at 45°, notched there.
    let area = r3d_geom::polygon::signed_area(&quarter.outline.polyline()).abs();
    assert!(
        (area - std::f64::consts::PI * 100.0 * 100.0 / 4.0).abs() < 2.0,
        "{area}"
    );
    let d = DVec2::new(1.0, 1.0).normalize() * 100.0;
    assert_eq!(quarter.notches.len(), 1);
    assert!(quarter.notches[0].distance(d) < 0.05);
    assert_eq!(grain_angle_deg(quarter.grain.unwrap()), 0.0);
    assert_eq!(quarter.seam_allowance_mm, 10.0);
    // The mirrored point B is at x = -100.
    let wedge = vertices(&e.pieces[1].1);
    assert!(has(&wedge, DVec2::new(-100.0, 0.0)));
}

/// A half piece against a fold line (Valentina's mirror line).
const FOLD: &str = r##"<pattern>
  <version>1.0.0</version>
  <unit>mm</unit>
  <draw name="F">
    <calculation>
      <point type="single" id="1" name="A" x="0" y="0"/>
      <point type="endLine" id="2" name="B" basePoint="1" length="50" angle="0"/>
      <point type="endLine" id="3" name="C" basePoint="2" length="100" angle="270"/>
      <point type="endLine" id="4" name="D" basePoint="1" length="100" angle="270"/>
    </calculation>
    <modeling>
      <point type="modeling" id="11" idObject="1"/>
      <point type="modeling" id="12" idObject="2"/>
      <point type="modeling" id="13" idObject="3"/>
      <point type="modeling" id="14" idObject="4"/>
    </modeling>
    <details>
      <detail id="20" version="2" name="Back" showFullPiece="true">
        <nodes>
          <node type="NodePoint" idObject="11"/>
          <node type="NodePoint" idObject="12"/>
          <node type="NodePoint" idObject="13"/>
          <node type="NodePoint" idObject="14"/>
        </nodes>
        <mirrorLine p1="14" p2="11"/>
      </detail>
    </details>
  </draw>
</pattern>"##;

#[test]
fn a_fold_line_unfolds_the_piece() {
    let e = evaluate(FOLD.as_bytes(), &Values::new()).unwrap();
    let back = &e.pieces[0].1;
    let area = r3d_geom::polygon::signed_area(&back.outline.polyline()).abs();
    assert!((area - 10_000.0).abs() < 1e-6, "{area}");
    assert!(has(&vertices(back), DVec2::new(-50.0, -100.0)));
}

#[test]
fn a_parallel_line_is_refused_with_the_step() {
    let bad = FOLD.replace(
        r#"<point type="endLine" id="4" name="D" basePoint="1" length="100" angle="270"/>"#,
        r#"<point type="lineIntersect" id="4" name="D" p1Line1="1" p2Line1="2" p1Line2="3" p2Line2="3"/>"#,
    );
    assert!(matches!(
        evaluate(bad.as_bytes(), &Values::new()),
        Err(ValError::Construction { id: 4, .. })
    ));
}
