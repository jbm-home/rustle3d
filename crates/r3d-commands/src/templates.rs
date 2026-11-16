//! Starter garments drafted from body measurements (`docs/specs/11`).
//!
//! Each template is a recipe: given a few sizes (cm) it creates the panels,
//! sews them, places them around the body and picks a fabric, through the
//! ordinary commands, so the result is an ordinary, editable project. The
//! drafts are our own, from general garment-making knowledge: simple
//! blocks with ease instead of darts, proportions taken from the chest or
//! the stature.

use r3d_core::{AnyId, Around, Body, EdgeId, Fabric, PanelId, Transaction};
use r3d_geom::{CubicBez, DVec2};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::commands::{
    CurveSpec, EdgeSetCurve, GarmentSetFabric, NotchAdd, PanelCreatePolygon, PanelMirror,
    PanelPlace, PanelSetSeamAllowance, PinAdd, ProjectRename, SeamAdd, SeamSideSpec,
    mirrored_source, waist_height_cm,
};
use crate::error::{CommandError, positive};
use crate::spec::CommandSpec;

/// The starter garments.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Template {
    /// A straight skirt: a front and two back halves, close at the hips.
    StraightSkirt,
    /// An A-line skirt: the straight skirt flared towards the hem.
    ALineSkirt,
    /// A T-shirt: front, back and two short sleeves.
    TShirt,
    /// A sleeveless dress: front and back from the neck to the knee,
    /// shaped at the waist, flared at the hem.
    Dress,
    /// A tote bag: two rectangles with handles, hanging from two pins.
    ToteBag,
}

impl Template {
    pub const ALL: [Template; 5] = [
        Template::TShirt,
        Template::Dress,
        Template::StraightSkirt,
        Template::ALineSkirt,
        Template::ToteBag,
    ];

    /// Stable identifier, as in JSON.
    pub fn id(self) -> &'static str {
        match self {
            Template::StraightSkirt => "straight_skirt",
            Template::ALineSkirt => "a_line_skirt",
            Template::ToteBag => "tote_bag",
            Template::TShirt => "t_shirt",
            Template::Dress => "dress",
        }
    }

    /// Default project name (English; front-ends translate by `id`).
    fn title(self) -> &'static str {
        match self {
            Template::StraightSkirt => "Straight skirt",
            Template::ALineSkirt => "A-line skirt",
            Template::ToteBag => "Tote bag",
            Template::TShirt => "T-shirt",
            Template::Dress => "Dress",
        }
    }

    /// The sizes this template reads.
    fn uses(self) -> &'static [&'static str] {
        match self {
            Template::StraightSkirt | Template::ALineSkirt => {
                &["waist_cm", "hips_cm", "length_cm", "ease_cm"]
            }
            Template::TShirt => &["chest_cm", "length_cm", "sleeve_cm", "ease_cm"],
            Template::Dress => &["chest_cm", "waist_cm", "hips_cm", "length_cm", "ease_cm"],
            Template::ToteBag => &["width_cm", "length_cm"],
        }
    }
}

/// Sizes for a template (cm). Any left out come from the avatar's
/// measurements, or from a standard size without an avatar.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Sizes {
    /// Body waist (skirts).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub waist_cm: Option<f64>,
    /// Body hips (skirts).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hips_cm: Option<f64>,
    /// Body chest or bust (tops).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chest_cm: Option<f64>,
    /// Garment length: waist to hem for skirts, neck to hem for tops.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub length_cm: Option<f64>,
    /// Width of a flat item (bags).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width_cm: Option<f64>,
    /// Sleeve length, from the top of the shoulder.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sleeve_cm: Option<f64>,
    /// Room added to the body measurements (hips for skirts, chest for
    /// tops).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ease_cm: Option<f64>,
}

/// Measurements used without an avatar: a standard women's M.
const STANDARD: Body = Body {
    masculinity: 0.0,
    height_cm: 168.0,
    bust_cm: 90.0,
    waist_cm: 73.0,
    hips_cm: 98.0,
};

/// Garment proportions, as fractions of stature (general anthropometry
/// for adults, in the base body's A pose).
const NECK_HEIGHT: f64 = 0.855;
/// Tops start this much above the neck (cm): their shoulder seams then
/// close over the shoulders and settle on them, instead of being pulled
/// together through the shoulders and sliding off.
const TOP_LIFT_CM: f64 = 4.0;
const SHOULDER_HEIGHT: f64 = 0.80;
const HIP_DEPTH: f64 = 0.115;

/// Gap between pieces on the pattern board (mm).
const BOARD_GAP: f64 = 80.0;
const SEAM_ALLOWANCE_MM: f64 = 10.0;

impl Sizes {
    /// Every size `template` reads, filled in from `body`.
    pub fn resolve(self, template: Template, body: &Body) -> Sizes {
        let h = body.height_cm;
        let (length, ease) = match template {
            Template::StraightSkirt => (0.33 * h, 2.0),
            Template::ALineSkirt => (0.36 * h, 2.0),
            Template::TShirt => (0.39 * h, 10.0),
            Template::Dress => (0.55 * h, 8.0),
            Template::ToteBag => (40.0, 0.0),
        };
        let all = Sizes {
            waist_cm: Some(self.waist_cm.unwrap_or(body.waist_cm)),
            hips_cm: Some(self.hips_cm.unwrap_or(body.hips_cm)),
            chest_cm: Some(self.chest_cm.unwrap_or(body.bust_cm)),
            length_cm: Some(self.length_cm.unwrap_or(round1(length))),
            width_cm: Some(self.width_cm.unwrap_or(38.0)),
            sleeve_cm: Some(self.sleeve_cm.unwrap_or(round1(0.12 * h))),
            ease_cm: Some(self.ease_cm.unwrap_or(ease)),
        };
        let keep = |name: &str, v: Option<f64>| v.filter(|_| template.uses().contains(&name));
        Sizes {
            waist_cm: keep("waist_cm", all.waist_cm),
            hips_cm: keep("hips_cm", all.hips_cm),
            chest_cm: keep("chest_cm", all.chest_cm),
            length_cm: keep("length_cm", all.length_cm),
            width_cm: keep("width_cm", all.width_cm),
            sleeve_cm: keep("sleeve_cm", all.sleeve_cm),
            ease_cm: keep("ease_cm", all.ease_cm),
        }
    }
}

fn round1(x: f64) -> f64 {
    (x * 10.0).round() / 10.0
}

/// A template as front-ends list it: its sizes resolved for a body.
#[derive(Clone, Debug, Serialize)]
pub struct TemplateInfo {
    pub template: Template,
    pub title: &'static str,
    pub sizes: Sizes,
}

/// Every template with its default sizes for `body` (or the standard
/// size).
pub fn list(body: Option<&Body>) -> Vec<TemplateInfo> {
    let body = body.unwrap_or(&STANDARD);
    Template::ALL
        .into_iter()
        .map(|t| TemplateInfo {
            template: t,
            title: t.title(),
            sizes: Sizes::default().resolve(t, body),
        })
        .collect()
}

/// A size the template reads (always resolved), in mm, checked positive.
fn mm(field: &'static str, cm: Option<f64>) -> Result<f64, CommandError> {
    let cm = cm.unwrap_or_default();
    positive(field, cm)?;
    Ok(cm * 10.0)
}

/// Draws panels side by side on the board, through commands.
struct Draft<'a, 'b> {
    tx: &'a mut Transaction<'b>,
    /// Left edge of the next panel on the board (mm).
    x: f64,
    created: Vec<AnyId>,
}

/// A drafted panel: its id and its contour edges, in drawing order.
struct Piece {
    id: PanelId,
    edges: Vec<EdgeId>,
}

impl Piece {
    fn edge(&self, k: usize) -> Result<EdgeId, CommandError> {
        self.edges
            .get(k)
            .copied()
            .ok_or_else(|| internal(format!("piece has no edge {k}")))
    }
}

fn internal(detail: String) -> CommandError {
    CommandError::Internal { detail }
}

impl Draft<'_, '_> {
    /// A panel from counter-clockwise `points` (mm, y up), each followed
    /// by the curve of the edge leaving it (`None` = straight; control
    /// points in the same frame). The panel is laid at the board cursor.
    fn panel(
        &mut self,
        name: &str,
        outline: &[(DVec2, Option<[DVec2; 2]>)],
    ) -> Result<Piece, CommandError> {
        let lo = outline.iter().fold(f64::MAX, |m, (p, _)| m.min(p.x));
        let hi = outline.iter().fold(f64::MIN, |m, (p, _)| m.max(p.x));
        let shift = DVec2::new(self.x - lo, 0.0);
        self.x += hi - lo + BOARD_GAP;
        let at = |p: DVec2| {
            let q = p + shift;
            [q.x, q.y]
        };
        let out = PanelCreatePolygon {
            name: Some(name.to_owned()),
            points: outline.iter().map(|(p, _)| at(*p)).collect(),
            grain_angle_deg: 90.0,
        }
        .apply(self.tx)?;
        let id = out
            .created
            .iter()
            .find_map(|a| match a {
                AnyId::Panel(p) => Some(*p),
                _ => None,
            })
            .ok_or_else(|| internal("no panel created".into()))?;
        self.created.push(id.into());
        let edges: Vec<EdgeId> = out
            .created
            .iter()
            .filter_map(|a| match a {
                AnyId::Edge(e) => Some(*e),
                _ => None,
            })
            .collect();
        for (edge, (_, curve)) in edges.iter().zip(outline) {
            if let Some([c1, c2]) = curve {
                EdgeSetCurve {
                    edge: *edge,
                    curve: CurveSpec::Cubic {
                        c1: at(*c1),
                        c2: at(*c2),
                    },
                }
                .apply(self.tx)?;
            }
        }
        PanelSetSeamAllowance {
            panel: id,
            mm: SEAM_ALLOWANCE_MM,
        }
        .apply(self.tx)?;
        Ok(Piece { id, edges })
    }

    /// A linked mirror copy of `source`, placed at the board cursor; its
    /// edges listed in the order of the source edges they mirror.
    fn mirror(&mut self, source: &Piece, name: &str) -> Result<Piece, CommandError> {
        let (min, max) = self
            .tx
            .project()
            .panel_bounding_box(source.id)
            .map_or((0.0, 0.0), |b| (b.min.x, b.max.x));
        // Reflected across x = o, the copy spans [2o - max, 2o - min]: it
        // starts at the cursor.
        let out = PanelMirror {
            panel: source.id,
            name: Some(name.to_owned()),
            axis_origin: Some([(self.x + max) / 2.0, 0.0]),
            axis_dir: None,
        }
        .apply(self.tx)?;
        self.x += max - min + BOARD_GAP;
        let id = out
            .created
            .first()
            .and_then(|a| match a {
                AnyId::Panel(p) => Some(*p),
                _ => None,
            })
            .ok_or_else(|| internal("no mirror created".into()))?;
        self.created.push(id.into());
        let p = self.tx.project();
        let copy_edges = p.panel(id).map(|pn| pn.contour.clone()).unwrap_or_default();
        let edges = source
            .edges
            .iter()
            .map(|src| {
                copy_edges
                    .iter()
                    .copied()
                    .find(|e| mirrored_source(p, *e).is_some_and(|(s, _)| s == *src))
                    .ok_or_else(|| internal(format!("no mirror of {src}")))
            })
            .collect::<Result<_, _>>()?;
        Ok(Piece { id, edges })
    }

    /// Sew `a` to `b`, walked in opposite directions: two pieces seen from
    /// outside, side by side, meet edge to edge that way.
    fn sew(&mut self, a: EdgeId, b: EdgeId) -> Result<(), CommandError> {
        self.sew_walking(a, b, true)
    }

    /// Sew `a` to `b`; `reversed`: walk `b` from its end.
    fn sew_walking(&mut self, a: EdgeId, b: EdgeId, reversed: bool) -> Result<(), CommandError> {
        let out = SeamAdd {
            a: SeamSideSpec {
                edge: a,
                reversed: None,
            },
            b: SeamSideSpec {
                edge: b,
                reversed: Some(reversed),
            },
        }
        .apply(self.tx)?;
        self.created.extend(out.created);
        Ok(())
    }

    fn place(
        &mut self,
        piece: &Piece,
        around: Around,
        angle_deg: f64,
        top_cm: f64,
    ) -> Result<(), CommandError> {
        self.place_at(piece, around, angle_deg, top_cm, 30.0)
    }

    fn place_at(
        &mut self,
        piece: &Piece,
        around: Around,
        angle_deg: f64,
        top_cm: f64,
        offset_mm: f64,
    ) -> Result<(), CommandError> {
        PanelPlace {
            panel: piece.id,
            around,
            angle_deg,
            top_cm,
            offset_mm,
            centre_x_mm: None,
        }
        .apply(self.tx)?;
        Ok(())
    }

    fn notch(&mut self, edge: EdgeId, offset_mm: f64) -> Result<(), CommandError> {
        let out = NotchAdd { edge, offset_mm }.apply(self.tx)?;
        self.created.extend(out.created);
        Ok(())
    }
}

fn p(x: f64, y: f64) -> DVec2 {
    DVec2::new(x, y)
}

/// Draft `template` into the project. Returns the created panels and
/// seams.
pub fn draft(
    tx: &mut Transaction<'_>,
    template: Template,
    sizes: Sizes,
) -> Result<Vec<AnyId>, CommandError> {
    let body = tx.project().avatar().map_or(STANDARD, |a| a.body);
    let sizes = sizes.resolve(template, &body);
    let fresh = tx.project().panels().is_empty();
    let x = tx
        .project()
        .panels()
        .keys()
        .filter_map(|id| tx.project().panel_bounding_box(*id))
        .fold(None, |m: Option<f64>, b| {
            Some(m.map_or(b.max.x, |m| m.max(b.max.x)))
        })
        .map_or(0.0, |right| right + BOARD_GAP);
    let mut d = Draft {
        tx,
        x,
        created: Vec::new(),
    };
    match template {
        Template::StraightSkirt => skirt(&mut d, &sizes, &body, 0.0)?,
        Template::ALineSkirt => skirt(&mut d, &sizes, &body, 0.2)?,
        Template::TShirt => t_shirt(&mut d, &sizes, &body)?,
        Template::Dress => dress(&mut d, &sizes, &body)?,
        Template::ToteBag => tote_bag(&mut d, &sizes)?,
    }
    let fabric = match template {
        Template::TShirt => Fabric::Jersey,
        Template::ToteBag => Fabric::Denim,
        _ => Fabric::Cotton,
    };
    GarmentSetFabric {
        garment: None,
        fabric,
    }
    .apply(d.tx)?;
    if fresh {
        ProjectRename {
            name: template.title().to_owned(),
        }
        .apply(d.tx)?;
    }
    Ok(d.created)
}

/// A skirt: a front and two back halves (the right one a mirror copy),
/// close over the hips, `flare` × length wider at each side of the hem.
fn skirt(d: &mut Draft, s: &Sizes, body: &Body, flare: f64) -> Result<(), CommandError> {
    let waist = mm("waist_cm", s.waist_cm)?;
    let hips = mm("hips_cm", s.hips_cm)?;
    let len = mm("length_cm", s.length_cm)?;
    let ease = s.ease_cm.unwrap_or_default() * 10.0;
    // Quarter widths: waist (no ease: the skirt hangs from it, and the
    // simulated body does not give like a real one under a waistband),
    // hips, hem.
    let a = waist / 4.0;
    let b = (hips + ease) / 4.0;
    let c = b + flare * len;
    let depth = (HIP_DEPTH * body.height_cm * 10.0).min(0.6 * len);
    let hip = p(b + flare * depth, len - depth);
    // Side seam, hem up to the waist: straight (or flared) to the hip line,
    // then curving in.
    let side = |sign: f64| {
        side_curve(
            p(sign * c, 0.0),
            p(sign * hip.x, hip.y),
            p(sign * a, len),
            depth,
        )
    };
    // The waist dips 8 mm at the centre, the hem stays straight.
    let waist_curve = |from: f64, to: f64| [p(from * 0.4, len - 8.0), p(to * 0.4, len - 8.0)];
    let front = d.panel(
        "Front",
        &[
            (p(-c, 0.0), None),
            (p(c, 0.0), Some(side(1.0))),
            (p(a, len), Some(waist_curve(a, -a))),
            (p(-a, len), Some(reverse(side(-1.0)))),
        ],
    )?;
    // Back left: centre back on its right (+x), side seam on its left.
    let back = d.panel(
        "Back left",
        &[
            (p(-c, 0.0), None),
            (p(0.0, 0.0), None),
            (
                p(0.0, len),
                Some([p(-0.3 * a, len), p(-0.6 * a, len - 4.0)]),
            ),
            (p(-a, len), Some(reverse(side(-1.0)))),
        ],
    )?;
    let side_len =
        d.tx.project()
            .edge_length(front.edge(1)?)
            .unwrap_or_default();
    for edge in [front.edge(1)?, front.edge(3)?, back.edge(3)?] {
        // Hip-line notch, measured from each edge's start.
        let from_start = if edge == front.edge(1)? {
            side_len - depth
        } else {
            depth
        };
        d.notch(edge, from_start.clamp(1.0, side_len - 1.0))?;
    }
    let right = d.mirror(&back, "Back right")?;
    d.sew(front.edge(1)?, back.edge(3)?)?;
    d.sew(front.edge(3)?, right.edge(3)?)?;
    d.sew(back.edge(1)?, right.edge(1)?)?;
    let top = waist_height_cm(d.tx.project())?;
    d.place(&front, Around::Body, 0.0, top)?;
    d.place(&back, Around::Body, 120.0, top)?;
    d.place(&right, Around::Body, 240.0, top)?;
    Ok(())
}

/// Control points of a side seam from the hem up to the waist: straight
/// (or flared) up to the hip line, then curving in to reach the waist
/// nearly upright, so the waistline stays a smooth ring where the panels
/// meet. `depth` is the waist-to-hip distance (mm).
fn side_curve(hem: DVec2, hip: DVec2, waist: DVec2, depth: f64) -> [DVec2; 2] {
    [
        hem.lerp(hip, 1.15),
        p(waist.x + 0.4 * (hip.x - waist.x), waist.y - 0.3 * depth),
    ]
}

fn reverse([c1, c2]: [DVec2; 2]) -> [DVec2; 2] {
    [c2, c1]
}

/// A T-shirt: front and back bodices (whole, symmetric) and two short
/// sleeves (the right one a mirror copy of the left).
fn t_shirt(d: &mut Draft, s: &Sizes, body: &Body) -> Result<(), CommandError> {
    let chest = mm("chest_cm", s.chest_cm)?;
    let len = mm("length_cm", s.length_cm)?;
    let sleeve = mm("sleeve_cm", s.sleeve_cm)?;
    let ease = s.ease_cm.unwrap_or_default() * 10.0;
    // Half widths from the chest: body, shoulder, neck.
    let w = (chest + ease) / 4.0;
    let shoulder = 0.21 * chest;
    let neck = chest / 12.0;
    let drop = 40.0;
    let armhole = 0.22 * chest + 20.0;
    let y_s = len - drop;
    let y_a = len - armhole;
    let bodice = |d: &mut Draft, name: &str, neck_depth: f64| {
        let k = 4.0 / 3.0 * neck_depth;
        // Armhole: leaves the underarm going in, reaches the shoulder
        // nearly upright.
        let hole = |sign: f64| {
            [
                p(sign * (shoulder + 0.3 * (w - shoulder)), y_a),
                p(sign * shoulder, y_a + 0.45 * (y_s - y_a)),
            ]
        };
        d.panel(
            name,
            &[
                (p(-w, 0.0), None),
                (p(w, 0.0), None),
                (p(w, y_a), Some(hole(1.0))),
                (p(shoulder, y_s), None),
                (p(neck, len), Some([p(neck, len - k), p(-neck, len - k)])),
                (p(-neck, len), None),
                (p(-shoulder, y_s), Some(reverse(hole(-1.0)))),
                (p(-w, y_a), None),
            ],
        )
    };
    let front = bodice(d, "Front", neck + 10.0)?;
    let back = bodice(d, "Back", 20.0)?;
    // The sleeve cap matches the armhole's length: its height is found by
    // bisection.
    let hole_len =
        d.tx.project()
            .edge_length(front.edge(2)?)
            .unwrap_or_default()
            .max(
                d.tx.project()
                    .edge_length(back.edge(2)?)
                    .unwrap_or_default(),
            );
    let half = (0.33 * chest + 60.0) / 2.0;
    let hem = half - 15.0;
    let cap = |h: f64| {
        let top = p(0.0, sleeve);
        let under = p(half, sleeve - h);
        [
            under,
            p(0.6 * half, sleeve - 0.35 * h),
            p(0.35 * half, sleeve),
            top,
        ]
    };
    let (mut lo, mut hi) = (10.0, sleeve * 0.9);
    for _ in 0..40 {
        let mid = (lo + hi) / 2.0;
        let [a, b, c, e] = cap(mid);
        if CubicBez::new(a, b, c, e).arc_length(0.01) < hole_len {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    let [under, c1, c2, top] = cap(lo);
    let mirror_x = |q: DVec2| p(-q.x, q.y);
    let left = d.panel(
        "Sleeve left",
        &[
            (p(-hem, 0.0), None),
            (p(hem, 0.0), None),
            (under, Some([c1, c2])),
            (top, Some([mirror_x(c2), mirror_x(c1)])),
            (mirror_x(under), None),
        ],
    )?;
    let right = d.mirror(&left, "Sleeve right")?;
    // Seen from outside, the front's +x side is the body's left, the
    // back's +x side its right; a left sleeve's +x side faces the back.
    d.sew(front.edge(1)?, back.edge(7)?)?;
    d.sew(front.edge(7)?, back.edge(1)?)?;
    d.sew(front.edge(3)?, back.edge(5)?)?;
    d.sew(front.edge(5)?, back.edge(3)?)?;
    d.sew(left.edge(3)?, front.edge(2)?)?;
    d.sew(left.edge(2)?, back.edge(6)?)?;
    d.sew(right.edge(3)?, front.edge(6)?)?;
    d.sew(right.edge(2)?, back.edge(2)?)?;
    d.sew(left.edge(1)?, left.edge(4)?)?;
    d.sew(right.edge(1)?, right.edge(4)?)?;
    let h = body.height_cm;
    d.place(&front, Around::Body, 0.0, NECK_HEIGHT * h + TOP_LIFT_CM)?;
    d.place(&back, Around::Body, 180.0, NECK_HEIGHT * h + TOP_LIFT_CM)?;
    d.place(&left, Around::LeftArm, 0.0, SHOULDER_HEIGHT * h)?;
    d.place(&right, Around::RightArm, 0.0, SHOULDER_HEIGHT * h)?;
    Ok(())
}

/// A sleeveless dress: front and back whole, the T-shirt's neckline,
/// shoulders and armholes, taken in at the waist, flared below the hips.
/// How far a dress's side seam sits towards the back (mm per quarter).
const SIDE_TO_BACK_MM: f64 = 10.0;

fn dress(d: &mut Draft, s: &Sizes, body: &Body) -> Result<(), CommandError> {
    let chest = mm("chest_cm", s.chest_cm)?;
    let waist = mm("waist_cm", s.waist_cm)?;
    let hips = mm("hips_cm", s.hips_cm)?;
    let len = mm("length_cm", s.length_cm)?;
    let ease = s.ease_cm.unwrap_or_default() * 10.0;
    let h = body.height_cm;
    // Quarter widths: chest, waist (twice the ease: it hangs from the
    // shoulders, gently shaped, not fitted), hips, hem.
    let w = (chest + ease) / 4.0;
    let a = (waist + 2.0 * ease) / 4.0;
    let b = (hips + ease) / 4.0;
    let shoulder = 0.21 * chest;
    let neck = chest / 12.0;
    let y_s = len - 40.0;
    let y_a = len - (0.22 * chest + 20.0);
    // Neck to waist on the body, then waist to hips.
    let y_w = (len - (NECK_HEIGHT * h - waist_height_cm(d.tx.project())?) * 10.0).max(0.4 * len);
    let depth = (HIP_DEPTH * h * 10.0).min(0.5 * y_w);
    let flare = 0.12;
    // The side seam sits 1 cm towards the back, as on a basic block: the
    // front, over the bust, is 4 cm wider around than the back, which
    // would otherwise hang in folds. The whole side moves, so both sides
    // of each seam keep the same length.
    let piece = |d: &mut Draft, name: &str, neck_depth: f64, shift: f64| {
        let (w, a, b) = (w + shift, a + shift, b + shift);
        let c = b + flare * (y_w - depth);
        let hip = p(b, y_w - depth);
        let k = 4.0 / 3.0 * neck_depth;
        let hole = |sign: f64| {
            [
                p(sign * (shoulder + 0.3 * (w - shoulder)), y_a),
                p(sign * shoulder, y_a + 0.45 * (y_s - y_a)),
            ]
        };
        let side = |sign: f64| {
            side_curve(
                p(sign * c, 0.0),
                p(sign * hip.x, hip.y),
                p(sign * a, y_w),
                depth,
            )
        };
        d.panel(
            name,
            &[
                (p(-c, 0.0), None),
                (p(c, 0.0), Some(side(1.0))),
                (p(a, y_w), None),
                (p(w, y_a), Some(hole(1.0))),
                (p(shoulder, y_s), None),
                (p(neck, len), Some([p(neck, len - k), p(-neck, len - k)])),
                (p(-neck, len), None),
                (p(-shoulder, y_s), Some(reverse(hole(-1.0)))),
                (p(-w, y_a), None),
                (p(-a, y_w), Some(reverse(side(-1.0)))),
            ],
        )
    };
    let front = piece(d, "Front", neck + 10.0, SIDE_TO_BACK_MM)?;
    let back = piece(d, "Back", 20.0, -SIDE_TO_BACK_MM)?;
    // Edges: 0 hem, 1–2 right side (hips, waist to underarm), 3 right
    // armhole, 4 right shoulder, 5 neckline, 6 left shoulder, 7 left
    // armhole, 8–9 left side. The front's right is the body's left, the
    // back's right the body's right.
    for (f, b) in [(1, 9), (2, 8), (4, 6), (6, 4), (8, 2), (9, 1)] {
        d.sew(front.edge(f)?, back.edge(b)?)?;
    }
    d.place(&front, Around::Body, 0.0, NECK_HEIGHT * h + TOP_LIFT_CM)?;
    d.place(&back, Around::Body, 180.0, NECK_HEIGHT * h + TOP_LIFT_CM)?;
    Ok(())
}

/// Where the tote bag hangs: in front of the body (m from its axis), its
/// handles' tops at this height (cm), its two layers this far apart (mm).
const TOTE_OUT_MM: f64 = 420.0;
const TOTE_TOP_CM: f64 = 125.0;
const TOTE_LAYERS_MM: f64 = 30.0;

/// A tote bag: front and back alike, each a rectangle with two handle
/// tabs; sewn at the sides, the bottom and the tabs' tops, and hung by its
/// handles from two pins in front of the body, as from hooks.
fn tote_bag(d: &mut Draft, s: &Sizes) -> Result<(), CommandError> {
    let w = mm("width_cm", s.width_cm)? / 2.0;
    let h = mm("length_cm", s.length_cm)?;
    // Handles: tabs `tab` wide, `handle` high, `inset` in from the sides.
    let (tab, handle, inset) = (40.0, 280.0, (0.22 * 2.0 * w).max(40.0));
    let (x0, x1) = (w - inset - tab, w - inset);
    let top = h + handle;
    let outline = [
        (p(-w, 0.0), None),
        (p(w, 0.0), None),
        (p(w, h), None),
        (p(x1, h), None),
        (p(x1, top), None),
        (p(x0, top), None),
        (p(x0, h), None),
        (p(-x0, h), None),
        (p(-x0, top), None),
        (p(-x1, top), None),
        (p(-x1, h), None),
        (p(-w, h), None),
    ];
    let front = d.panel("Front", &outline)?;
    let back = d.panel("Back", &outline)?;
    // Edges: 0 bottom, 1 right side, 4 right tab's top, 8 left tab's top,
    // 11 left side. Both layers face the same way: sewn walking alike.
    for k in [0, 1, 4, 8, 11] {
        d.sew_walking(front.edge(k)?, back.edge(k)?, false)?;
    }
    d.place_at(&front, Around::Flat, 0.0, TOTE_TOP_CM, TOTE_OUT_MM)?;
    d.place_at(
        &back,
        Around::Flat,
        0.0,
        TOTE_TOP_CM,
        TOTE_OUT_MM - TOTE_LAYERS_MM,
    )?;
    // The pins hold the front's tab tops where the bag starts (the flat
    // layout: x across, y up, z out), so it hangs as it was laid.
    let left =
        d.tx.project()
            .panel_bounding_box(front.id)
            .map_or(0.0, |b| b.min.x);
    for x in [-(x0 + x1) / 2.0, (x0 + x1) / 2.0] {
        let out = PinAdd {
            panel: front.id,
            at: [left + w + x, top - 10.0],
            position: [x / 1000.0, TOTE_TOP_CM / 100.0 - 0.01, TOTE_OUT_MM / 1000.0],
        }
        .apply(d.tx)?;
        d.created.extend(out.created);
    }
    Ok(())
}
