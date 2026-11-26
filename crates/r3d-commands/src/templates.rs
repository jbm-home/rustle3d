//! Starter garments drafted from body measurements (`docs/specs/11`).
//!
//! Each template is a recipe: given a few sizes (cm) it creates the panels,
//! sews them, places them around the body and picks a fabric, through the
//! ordinary commands, so the result is an ordinary, editable project. The
//! drafts are our own, from general garment-making knowledge: simple
//! blocks with ease instead of darts, proportions taken from the chest or
//! the stature.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU32, Ordering};

use rayon::prelude::*;

use r3d_core::detail::ELASTIC_RATIO;
use r3d_core::{
    AnyId, Around, Avatar, Body, Edge, EdgeCurve, EdgeId, Fabric, Garment, GarmentId,
    InternalLineId, Measure, PanelId, Point, Project, TemplateRef, Transaction,
};
use r3d_geom::{CubicBez, DVec2};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::commands::{
    ButtonAdd, CurveSpec, EdgeSetCurve, EdgeSplit, ElasticAdd, GarmentAdd, GarmentRename,
    GarmentSetFabric, InterfacingAdd, InternalLineAdd, NotchAdd, PanelCreatePolygon, PanelMirror,
    PanelPlace, PanelSetGarment, PanelSetSeamAllowance, PanelUnlink, PinAdd, SeamAdd, SeamSideSpec,
    SegmentSpec, TopstitchAdd, ZipperAdd, mirrored_source, waist_height_cm,
};
use crate::drape::arm_drop;
use crate::error::{CommandError, positive};
use crate::library::PatternKind;
use crate::progress::{Progress, Status};
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
    /// A shirt: two fronts, a back, long sleeves with cuffs and a stand
    /// collar.
    Shirt,
    /// Trousers: a front and a back for each leg, from the waist to the
    /// ankle.
    Trousers,
    /// A hoodie: the T-shirt's block, looser, with long sleeves gathered
    /// at the wrists, an elastic hem and a hood in two halves.
    Hoodie,
    /// Joggers: the trousers' block in jersey, pulled on: an elastic
    /// waist and elastic ankles, no fly.
    Joggers,
    /// Jeans: the trousers' block in denim with five pockets, a back
    /// yoke, a fly and a waistband with belt loops.
    Jeans,
    /// Shorts: the trousers' block ending on the thigh.
    Shorts,
    /// A sleeveless dress: front and back from the neck to the knee,
    /// shaped at the waist, flared at the hem.
    Dress,
    /// A tote bag: two rectangles with handles, hanging from two pins.
    ToteBag,
    /// A sweatshirt: the hoodie without its hood.
    Sweatshirt,
    /// A baby's bodysuit in jersey: the T-shirt's block going on under
    /// the crotch, where front and back close, with leg openings.
    Bodysuit,
    /// A pyjama: its bottoms, and its top worn over them (a set: two
    /// garments).
    Pyjama,
    /// A pyjama's top in jersey: loose, long sleeves gathered at the
    /// wrists.
    PyjamaTop,
    /// A pyjama's bottoms in jersey: the joggers' block, looser, pulled
    /// on by an elastic waist, open at the ankles.
    PyjamaBottoms,
    /// A jogging suit: joggers, and a sweatshirt worn over them (a set:
    /// two garments).
    JoggingSuit,
}

impl Template {
    pub const ALL: [Template; 17] = [
        Template::TShirt,
        Template::Shirt,
        Template::Trousers,
        Template::Hoodie,
        Template::Joggers,
        Template::Jeans,
        Template::Shorts,
        Template::Dress,
        Template::StraightSkirt,
        Template::ALineSkirt,
        Template::ToteBag,
        Template::Sweatshirt,
        Template::Bodysuit,
        Template::Pyjama,
        Template::PyjamaTop,
        Template::PyjamaBottoms,
        Template::JoggingSuit,
    ];

    /// The templates drafted to add it, each as a garment, in the order
    /// they are worn (a set's bottoms, then its top over them).
    pub fn parts(self) -> Vec<Template> {
        match self {
            Template::Pyjama => vec![Template::PyjamaBottoms, Template::PyjamaTop],
            Template::JoggingSuit => vec![Template::Joggers, Template::Sweatshirt],
            t => vec![t],
        }
    }

    /// Stable identifier, as in JSON.
    pub fn id(self) -> &'static str {
        match self {
            Template::StraightSkirt => "straight_skirt",
            Template::ALineSkirt => "a_line_skirt",
            Template::ToteBag => "tote_bag",
            Template::TShirt => "t_shirt",
            Template::Shirt => "shirt",
            Template::Trousers => "trousers",
            Template::Hoodie => "hoodie",
            Template::Joggers => "joggers",
            Template::Jeans => "jeans",
            Template::Shorts => "shorts",
            Template::Dress => "dress",
            Template::Sweatshirt => "sweatshirt",
            Template::Bodysuit => "bodysuit",
            Template::Pyjama => "pyjama",
            Template::PyjamaTop => "pyjama_top",
            Template::PyjamaBottoms => "pyjama_bottoms",
            Template::JoggingSuit => "jogging_suit",
        }
    }

    /// Where it sits in the pattern library.
    pub fn kind(self) -> PatternKind {
        match self {
            Template::TShirt | Template::Shirt | Template::Hoodie | Template::Sweatshirt => {
                PatternKind::Tops
            }
            Template::Trousers | Template::Joggers | Template::Jeans | Template::Shorts => {
                PatternKind::Bottoms
            }
            Template::Dress | Template::StraightSkirt | Template::ALineSkirt => {
                PatternKind::DressesAndSkirts
            }
            Template::ToteBag => PatternKind::Accessories,
            Template::Bodysuit
            | Template::Pyjama
            | Template::PyjamaTop
            | Template::PyjamaBottoms
            | Template::JoggingSuit => PatternKind::Children,
        }
    }

    /// Default garment name (English; front-ends translate by `id`).
    pub fn title(self) -> &'static str {
        match self {
            Template::StraightSkirt => "Straight skirt",
            Template::ALineSkirt => "A-line skirt",
            Template::ToteBag => "Tote bag",
            Template::TShirt => "T-shirt",
            Template::Shirt => "Shirt",
            Template::Trousers => "Trousers",
            Template::Hoodie => "Hoodie",
            Template::Joggers => "Joggers",
            Template::Jeans => "Jeans",
            Template::Shorts => "Shorts",
            Template::Dress => "Dress",
            Template::Sweatshirt => "Sweatshirt",
            Template::Bodysuit => "Bodysuit",
            Template::Pyjama => "Pyjama",
            Template::PyjamaTop => "Pyjama top",
            Template::PyjamaBottoms => "Pyjama bottoms",
            Template::JoggingSuit => "Jogging suit",
        }
    }

    /// The sizes this template reads.
    fn uses(self) -> &'static [&'static str] {
        match self {
            Template::StraightSkirt
            | Template::ALineSkirt
            | Template::Trousers
            | Template::Joggers
            | Template::Jeans
            | Template::Shorts
            | Template::PyjamaBottoms => &["waist_cm", "hips_cm", "length_cm", "ease_cm"],
            Template::TShirt
            | Template::Shirt
            | Template::Hoodie
            | Template::Sweatshirt
            | Template::PyjamaTop => &["chest_cm", "length_cm", "sleeve_cm", "ease_cm"],
            Template::Dress => &["chest_cm", "waist_cm", "hips_cm", "length_cm", "ease_cm"],
            Template::ToteBag => &["width_cm", "length_cm"],
            Template::Bodysuit => &["chest_cm", "hips_cm", "length_cm", "sleeve_cm", "ease_cm"],
            Template::Pyjama | Template::JoggingSuit => &[
                "chest_cm",
                "waist_cm",
                "hips_cm",
                "length_cm",
                "sleeve_cm",
                "ease_cm",
            ],
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
    /// Garment length: waist to hem for skirts and trousers, neck to hem
    /// for tops (a set's top; a bodysuit's to its crotch).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub length_cm: Option<f64>,
    /// Width of a flat item (bags).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width_cm: Option<f64>,
    /// Sleeve length, from the top of the shoulder (with the cuff, on a
    /// shirt).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sleeve_cm: Option<f64>,
    /// Room added to the body measurements (hips for skirts and trousers,
    /// chest for tops and a set's top).
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
    age_years: None,
};

/// The avatar templates are drafted for: the project's, or one of the
/// standard size without one.
pub(crate) fn avatar_of(p: &Project) -> Avatar {
    p.avatar().cloned().unwrap_or(Avatar {
        body: STANDARD,
        preset: None,
        mesh: None,
        measures: Default::default(),
    })
}

/// Garment proportions, as fractions of stature (general anthropometry
/// for adults, in the base body's A pose).
const NECK_HEIGHT: f64 = 0.855;
/// Tops start this much above the neck (cm): their shoulder seams then
/// close over the shoulders and settle on them, instead of being pulled
/// together through the shoulders and sliding off.
const TOP_LIFT_CM: f64 = 4.0;
const SHOULDER_HEIGHT: f64 = 0.80;
const HIP_DEPTH: f64 = 0.115;
/// The crotch, as a fraction of stature.
const CROTCH_HEIGHT: f64 = 0.455;

/// Where garments sit on a body, as fractions of its stature: the adult
/// ones above ([`NECK_HEIGHT`], [`SHOULDER_HEIGHT`], [`CROTCH_HEIGHT`]),
/// lowered on a child as the body model's child is lower there than its
/// adult (its head is larger).
#[derive(Clone, Copy, Debug)]
struct Stature {
    neck: f64,
    shoulder: f64,
    crotch: f64,
}

impl Stature {
    fn of(body: &Body) -> Stature {
        let adult = Stature {
            neck: NECK_HEIGHT,
            shoulder: SHOULDER_HEIGHT,
            crotch: CROTCH_HEIGHT,
        };
        let Some(age) = body.age_years else {
            return adult;
        };
        let Ok(model) = crate::queries::body_model() else {
            return adult;
        };
        let m = body.masculinity as f32;
        let proportions = |age| r3d_avatar::fit::proportions(model, m, age);
        match (proportions(Some(age as f32)), proportions(None)) {
            (Some(child), Some(grown)) => Stature {
                neck: NECK_HEIGHT * f64::from(child.neck / grown.neck),
                shoulder: SHOULDER_HEIGHT * f64::from(child.shoulder / grown.shoulder),
                crotch: CROTCH_HEIGHT * f64::from(child.crotch / grown.crotch),
            },
            _ => adult,
        }
    }
}

/// Gap between pieces on the pattern board (mm).
const BOARD_GAP: f64 = 80.0;
/// What a shirt-like pattern takes from the upper body (mm), measured on
/// the avatar (or the standard body without one): proportions of the
/// chest made a size 52's shoulders 26 cm wide each side for 19 on the
/// body, and its neckline as wide.
struct Upper {
    body: r3d_avatar::fit::UpperBody,
    /// What the adult eases and allowances below are multiplied by
    /// ([`ease_scale`]).
    scale: f64,
}

/// The chest the adult eases below were set for (cm).
const ADULT_CHEST_CM: f64 = 90.0;

/// The eases and allowances of an upper-body pattern, set in mm for an
/// adult, scale with a child's chest: an adult's 7 cm under the arm and
/// 6 cm across the sleeve made a baby's bodysuit boxy, its armholes deep
/// and its sleeves puffed. An adult's are kept as they are.
fn ease_scale(body: &Body) -> f64 {
    match body.age_years {
        Some(_) => (body.bust_cm / ADULT_CHEST_CM).clamp(0.3, 1.0),
        None => 1.0,
    }
}

/// From the shoulder joint to the shoulder's tip (mm).
const JOINT_TO_TIP_MM: f64 = 20.0;
/// Ease at the shoulder's tip, at the neck (on its radius) and under
/// the arm (mm).
const SHOULDER_EASE_MM: f64 = 10.0;
const NECK_EASE_MM: f64 = 10.0;
const ARMHOLE_EASE_MM: f64 = 70.0;
/// From the neck point down to the shoulder's tip (mm).
const SHOULDER_DROP_MM: f64 = 40.0;
/// The body model's shoulders do not widen with its weight; a size chart
/// widens them by about 0.4 cm per size (4 cm of chest): each shoulder
/// takes this many mm per mm of chest over the body's at neutral weight.
const SHOULDER_PER_CHEST: f64 = 0.05;

impl Upper {
    /// The upper body of `p`'s avatar: its measured body fitted (an
    /// imported body's measurements, too).
    fn of(p: &Project) -> Result<Self, CommandError> {
        // The body follows a typed neck and shoulder width (spec 29).
        let avatar = avatar_of(p);
        let fit = crate::queries::fitted(&avatar.body, &avatar.measures)?;
        r3d_avatar::fit::upper_body(crate::queries::body_model()?, &fit)
            .map(|body| Self {
                body,
                scale: ease_scale(&avatar.body),
            })
            .ok_or_else(|| internal("the body's shoulders and neck were not found".into()))
    }

    /// From the centre line to the shoulder's tip, for a pattern of chest
    /// `chest` (mm).
    fn shoulder_half(&self, chest: f64) -> f64 {
        let u = &self.body;
        f64::from(u.shoulder_joint_cm) * 10.0
            + (JOINT_TO_TIP_MM + SHOULDER_EASE_MM) * self.scale
            + SHOULDER_PER_CHEST * (chest - f64::from(u.neutral_bust_cm) * 10.0).max(0.0)
    }

    /// Half the neckline's width at the base of the neck.
    fn neck_half(&self) -> f64 {
        f64::from(self.body.neck_cm) * 10.0 / std::f64::consts::TAU + NECK_EASE_MM * self.scale
    }

    /// From the shoulder's tip down to the bottom of the armhole.
    fn armhole_depth(&self) -> f64 {
        f64::from(self.body.shoulder_top_cm - self.body.armpit_cm) * 10.0
            + ARMHOLE_EASE_MM * self.scale
    }

    /// How far the shoulder's tip lies below the neck point.
    fn shoulder_drop(&self) -> f64 {
        SHOULDER_DROP_MM * self.scale
    }
}

/// A shirt's sleeve (cuff included) ends this much short of the arm's
/// length (cm). The whole arm's length, 2 cm more than before on the
/// women's M, let a tucked shirt show through the trousers' waist after
/// the drape (`a_shirt_under_trousers_is_tucked_in`).
const LONG_SLEEVE_SHORT_CM: f64 = 2.0;

/// A T-shirt's sleeve, as a fraction of the arm's length.
const SHORT_SLEEVE: f64 = 0.4;

/// Trousers end this far above the floor by default (cm): their length
/// is the waist's height less this. A fraction of the stature left the
/// hem on the floor of one body and 19 cm above it on another.
const TROUSER_HEM_CM: f64 = 4.0;
/// Joggers end higher, their elastic cuffs at the ankles (cm).
const JOGGER_HEM_CM: f64 = 8.0;
/// Shorts end this far below the crotch by default (cm): above the knee;
/// on a leg shorter than [`SHORT_LEG_CM`] (a child's), in proportion.
const SHORTS_INSEAM_CM: f64 = 22.0;
const SHORT_LEG_CM: f64 = 72.0;

impl Sizes {
    /// Every size `template` reads, filled in from `avatar`, whose waist
    /// (its narrowest girth) is `waist_height_cm` above the floor.
    pub fn resolve(self, template: Template, avatar: &Avatar, waist_height_cm: f64) -> Sizes {
        let body = &avatar.body;
        let h = body.height_cm;
        let stature = Stature::of(body);
        let leg = (avatar.measure(Measure::InsideLeg).cm / SHORT_LEG_CM).min(1.0);
        // From the neck down to the crotch.
        let torso = (stature.neck - stature.crotch) * h;
        let (length, ease) = match template {
            Template::StraightSkirt => (0.33 * h, 2.0),
            Template::ALineSkirt => (0.36 * h, 2.0),
            Template::Trousers => (waist_height_cm - TROUSER_HEM_CM, 4.0),
            Template::Joggers => (waist_height_cm - JOGGER_HEM_CM, 8.0),
            Template::Jeans => (waist_height_cm - TROUSER_HEM_CM, 4.0),
            Template::Shorts => (
                waist_height_cm - stature.crotch * h + leg * SHORTS_INSEAM_CM,
                6.0,
            ),
            Template::Hoodie | Template::Sweatshirt | Template::JoggingSuit => (torso, 16.0),
            Template::PyjamaTop | Template::Pyjama => (torso + 1.0, 14.0),
            Template::PyjamaBottoms => (waist_height_cm - JOGGER_HEM_CM, 12.0),
            Template::TShirt => (0.39 * h, 10.0),
            Template::Shirt => (0.43 * h, 12.0),
            Template::Dress => (0.55 * h, 8.0),
            Template::ToteBag => (40.0, 0.0),
            // From the neck (where the top starts, lifted) down under the
            // crotch.
            Template::Bodysuit => (torso + TOP_LIFT_CM + 2.0, 6.0),
        };
        // A long sleeve ends just above the wrist, a short one at mid
        // upper arm.
        let arm = avatar.measure(Measure::ArmLength).cm;
        let sleeve = match template {
            Template::Shirt
            | Template::Hoodie
            | Template::Sweatshirt
            | Template::Pyjama
            | Template::PyjamaTop
            | Template::JoggingSuit => arm - LONG_SLEEVE_SHORT_CM,
            _ => SHORT_SLEEVE * arm,
        };
        let all = Sizes {
            waist_cm: Some(self.waist_cm.unwrap_or(body.waist_cm)),
            hips_cm: Some(self.hips_cm.unwrap_or(body.hips_cm)),
            chest_cm: Some(self.chest_cm.unwrap_or(body.bust_cm)),
            length_cm: Some(self.length_cm.unwrap_or(round1(length))),
            width_cm: Some(self.width_cm.unwrap_or(38.0)),
            sleeve_cm: Some(self.sleeve_cm.unwrap_or(round1(sleeve))),
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
    pub kind: PatternKind,
    pub sizes: Sizes,
}

/// Every template with its default sizes for `project`'s avatar (or the
/// standard size).
pub fn list(project: &Project) -> Result<Vec<TemplateInfo>, CommandError> {
    let avatar = avatar_of(project);
    let waist = waist_height_cm(project)?;
    Ok(Template::ALL
        .into_iter()
        .map(|t| TemplateInfo {
            template: t,
            title: t.title(),
            kind: t.kind(),
            sizes: Sizes::default().resolve(t, &avatar, waist),
        })
        .collect())
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
    /// Where each piece's draft x = 0 lies on the board (mm).
    origins: BTreeMap<PanelId, f64>,
    /// The least room each piece takes on the board, in drafting order:
    /// its extent around its origin (mm).
    room: &'a [(f64, f64)],
}

/// A drafted panel: its id, its contour edges in drawing order, and
/// where the draft's x = 0 lies on the board (mm), the draft reflected
/// for a mirror copy.
struct Piece {
    id: PanelId,
    edges: Vec<EdgeId>,
    origin_x: f64,
    mirrored: bool,
}

impl Piece {
    /// A point of the board in the draft (mm): [`Piece::board`] undone.
    fn draft(&self, b: DVec2) -> DVec2 {
        let x = b.x - self.origin_x;
        p(if self.mirrored { -x } else { x }, b.y)
    }

    /// A point of the draft (mm) on the board.
    fn board(&self, q: DVec2) -> [f64; 2] {
        let x = if self.mirrored { -q.x } else { q.x };
        [self.origin_x + x, q.y]
    }

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
        let (lo, hi) = match self.room.get(self.origins.len()) {
            Some((rl, rh)) => (lo.min(*rl), hi.max(*rh)),
            None => (lo, hi),
        };
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
            symmetric: false,
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
            mm: crate::commands::DEFAULT_SEAM_ALLOWANCE_MM,
        }
        .apply(self.tx)?;
        self.origins.insert(id, shift.x);
        Ok(Piece {
            id,
            edges,
            origin_x: shift.x,
            mirrored: false,
        })
    }

    /// A linked mirror copy of `source`, placed at the board cursor; its
    /// edges listed in the order of the source edges they mirror.
    fn mirror(&mut self, source: &Piece, name: &str) -> Result<Piece, CommandError> {
        let (min, max) = self
            .tx
            .project()
            .panel_bounding_box(source.id)
            .map_or((0.0, 0.0), |b| (b.min.x, b.max.x));
        // Reflected across x = o, the copy's origin is at 2o - origin and
        // it spans [2o - max, 2o - min] (or its room around its origin):
        // it starts at the cursor.
        let so = source.origin_x;
        let (lo, hi) = (so - max, so - min);
        let (lo, hi) = match self.room.get(self.origins.len()) {
            Some((rl, rh)) => (lo.min(*rl), hi.max(*rh)),
            None => (lo, hi),
        };
        let axis = (self.x - lo + so) / 2.0;
        let out = PanelMirror {
            panel: source.id,
            name: Some(name.to_owned()),
            axis_origin: Some([axis, 0.0]),
            axis_dir: None,
        }
        .apply(self.tx)?;
        self.x += hi - lo + BOARD_GAP;
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
        let origin_x = 2.0 * axis - source.origin_x;
        self.origins.insert(id, origin_x);
        Ok(Piece {
            id,
            edges,
            origin_x,
            mirrored: !source.mirrored,
        })
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
                edge: Some(a),
                ..SeamSideSpec::default()
            },
            b: SeamSideSpec {
                edge: Some(b),
                reversed: Some(reversed),
                ..SeamSideSpec::default()
            },
            ease_mm: None,
        }
        .apply(self.tx)?;
        self.created.extend(out.created);
        Ok(())
    }

    /// Sew two stretches given as `seam.add` takes them.
    fn sew_sides(&mut self, a: SeamSideSpec, b: SeamSideSpec) -> Result<(), CommandError> {
        let out = SeamAdd {
            a,
            b,
            ease_mm: None,
        }
        .apply(self.tx)?;
        self.created.extend(out.created);
        Ok(())
    }

    /// A closed zipper between two stretches, each from its bottom stop.
    fn zip(&mut self, a: SeamSideSpec, b: SeamSideSpec) -> Result<(), CommandError> {
        let out = ZipperAdd {
            a,
            b,
            open_mm: None,
        }
        .apply(self.tx)?;
        self.created.extend(out.created);
        Ok(())
    }

    /// A buttoned button at draft point `at` of `on`, through a
    /// buttonhole at `hole` of `hole_on` (its slit towards the piece's
    /// middle).
    fn button(
        &mut self,
        on: &Piece,
        at: DVec2,
        hole_on: &Piece,
        hole: DVec2,
    ) -> Result<(), CommandError> {
        let out = ButtonAdd {
            panel: on.id,
            at: on.board(at),
            diameter_mm: Some(BUTTON_MM),
            hole_panel: hole_on.id,
            hole_at: hole_on.board(hole),
            hole_angle_deg: None,
            closed: None,
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
        self.place_at(piece, around, angle_deg, top_cm, 30.0, None)
    }

    /// Place a piece with the pattern's vertical line `centre_x_mm` (by
    /// default the piece's horizontal centre) at `angle_deg`.
    fn place_at(
        &mut self,
        piece: &Piece,
        around: Around,
        angle_deg: f64,
        top_cm: f64,
        offset_mm: f64,
        centre_x_mm: Option<f64>,
    ) -> Result<(), CommandError> {
        PanelPlace {
            panel: piece.id,
            around,
            angle_deg,
            top_cm,
            offset_mm,
            centre_x_mm,
        }
        .apply(self.tx)?;
        Ok(())
    }

    /// A medium woven interfacing on the whole of `piece`.
    fn interface(&mut self, piece: PanelId) -> Result<(), CommandError> {
        let out = InterfacingAdd {
            panel: piece,
            area: None,
            fabric: None,
            weight: None,
        }
        .apply(self.tx)?;
        self.created.extend(out.created);
        Ok(())
    }

    /// `rows` rows of topstitching inside `edge`, the first `offset_mm`
    /// from it, 5 mm apart.
    fn topstitch(&mut self, edge: EdgeId, rows: u8, offset_mm: f64) -> Result<(), CommandError> {
        let out = TopstitchAdd {
            side: SeamSideSpec {
                edge: Some(edge),
                ..SeamSideSpec::default()
            },
            rows: Some(rows),
            offset_mm: Some(offset_mm),
            gap_mm: Some(5.0),
            stitch_mm: None,
            color: None,
            thread: None,
        }
        .apply(self.tx)?;
        self.created.extend(out.created);
        Ok(())
    }

    /// An elastic along `side`, gathering it to `ratio` × its length.
    fn elastic(&mut self, side: SeamSideSpec, ratio: f64) -> Result<(), CommandError> {
        let out = ElasticAdd {
            side,
            ratio: Some(ratio.clamp(ELASTIC_RATIO.0, ELASTIC_RATIO.1)),
            relaxed_mm: None,
            width_mm: None,
        }
        .apply(self.tx)?;
        self.created.extend(out.created);
        Ok(())
    }

    fn notch(&mut self, edge: EdgeId, offset_mm: f64) -> Result<(), CommandError> {
        let out = NotchAdd { edge, offset_mm }.apply(self.tx)?;
        self.created.extend(out.created);
        Ok(())
    }

    /// A piece drawn reflected (as the right pieces are) from `points` of
    /// the left pieces' draft, joined by straight edges: given clockwise,
    /// it comes counter-clockwise.
    fn reflected(&mut self, name: &str, points: &[DVec2]) -> Result<Piece, CommandError> {
        let outline: Vec<_> = points.iter().map(|q| (p(-q.x, q.y), None)).collect();
        let piece = self.panel(name, &outline)?;
        Ok(Piece {
            mirrored: true,
            ..piece
        })
    }

    /// An internal line on `piece` through draft points: from `start`,
    /// each segment to a point, straight or by its curve's controls.
    fn line(
        &mut self,
        piece: &Piece,
        start: DVec2,
        segments: &[(DVec2, Option<[DVec2; 2]>)],
    ) -> Result<InternalLineId, CommandError> {
        let out = InternalLineAdd {
            panel: piece.id,
            start: piece.board(start),
            segments: segments
                .iter()
                .map(|(to, curve)| match curve {
                    None => SegmentSpec::Line {
                        to: piece.board(*to),
                    },
                    Some([c1, c2]) => SegmentSpec::Cubic {
                        c1: piece.board(*c1),
                        c2: piece.board(*c2),
                        to: piece.board(*to),
                    },
                })
                .collect(),
        }
        .apply(self.tx)?;
        out.created
            .iter()
            .find_map(|a| match a {
                AnyId::InternalLine(l) => Some(*l),
                _ => None,
            })
            .ok_or_else(|| internal("no line created".into()))
    }

    /// Edges `ks` of `piece` as one seam side, in their order around it
    /// (a mirror copy lists its edges as its source's, which run the other
    /// way round it).
    fn run(&self, piece: &Piece, ks: &[usize]) -> Result<SeamSideSpec, CommandError> {
        let mut edges = ks
            .iter()
            .map(|k| piece.edge(*k))
            .collect::<Result<Vec<_>, _>>()?;
        let p = self.tx.project();
        let joined = edges.windows(2).all(|w| match w {
            [a, b] => p
                .edge(*a)
                .zip(p.edge(*b))
                .is_some_and(|(a, b)| a.end == b.start),
            _ => true,
        });
        if !joined {
            edges.reverse();
        }
        Ok(SeamSideSpec {
            edges,
            ..SeamSideSpec::default()
        })
    }

    /// Where `edge` of `piece` starts and ends, in the draft.
    fn ends(&self, piece: &Piece, edge: EdgeId) -> Option<(DVec2, DVec2)> {
        let p = self.tx.project();
        let e = p.edge(edge)?;
        Some((
            piece.draft(p.point(e.start)?.pos),
            piece.draft(p.point(e.end)?.pos),
        ))
    }

    /// Where a seam side of `piece` (whole edges or a whole line) starts
    /// and ends, in the draft, walked as it says.
    fn side_ends(&self, piece: &Piece, side: &SeamSideSpec) -> Option<(DVec2, DVec2)> {
        let p = self.tx.project();
        let (a, z) = match (side.line, side.edges.first(), side.edges.last()) {
            (Some(line), _, _) => {
                let l = p.internal_line(line)?;
                (l.start, l.segments.last().map_or(l.start, |s| s.end()))
            }
            (None, Some(first), Some(last)) => (
                p.point(p.edge(*first)?.start)?.pos,
                p.point(p.edge(*last)?.end)?.pos,
            ),
            _ => {
                let e = p.edge(side.edge?)?;
                (p.point(e.start)?.pos, p.point(e.end)?.pos)
            }
        };
        let (a, z) = (piece.draft(a), piece.draft(z));
        Some(if side.reversed == Some(true) {
            (z, a)
        } else {
            (a, z)
        })
    }

    /// Sew `a` of `a_piece` onto `b` of `b_piece` lying over it (a facing
    /// under a front, a pocket on a back), both drawn in the same draft:
    /// `b` walked from the end that lies on `a`'s start.
    fn overlay(
        &mut self,
        a_piece: &Piece,
        a: SeamSideSpec,
        b_piece: &Piece,
        mut b: SeamSideSpec,
    ) -> Result<(), CommandError> {
        let (Some((a0, _)), Some((b0, b1))) =
            (self.side_ends(a_piece, &a), self.side_ends(b_piece, &b))
        else {
            return Err(internal("a side without ends".into()));
        };
        if a0.distance(b1) < a0.distance(b0) {
            b.reversed = Some(b.reversed != Some(true));
        }
        self.sew_sides(a, b)
    }

    /// Sew two chains of edges that meet end to end on the body (a front
    /// and its facing against a back and its yoke), each listed from the
    /// hem up: stretch by stretch, each chain's length shared out in
    /// proportion.
    fn sew_chains(&mut self, a: &[EdgeId], b: &[EdgeId]) -> Result<(), CommandError> {
        // Each edge's length, and whether it runs up (from the hem).
        let chain =
            |d: &Self, edges: &[EdgeId]| -> Result<Vec<(EdgeId, f64, bool)>, CommandError> {
                edges
                    .iter()
                    .map(|e| {
                        let p = d.tx.project();
                        let edge = p.edge(*e).ok_or_else(|| internal(format!("no edge {e}")))?;
                        let y = |pt| p.point(pt).map_or(0.0, |q| q.pos.y);
                        Ok((
                            *e,
                            p.edge_length(*e).unwrap_or_default(),
                            y(edge.end) > y(edge.start),
                        ))
                    })
                    .collect()
            };
        let (a, b) = (chain(self, a)?, chain(self, b)?);
        let total = |c: &[(EdgeId, f64, bool)]| c.iter().map(|(_, l, _)| l).sum::<f64>();
        let (ta, tb) = (total(&a), total(&b));
        if ta <= 0.0 || tb <= 0.0 {
            return Err(internal("an empty chain".into()));
        }
        // Where each chain's edges meet, as shares of its length.
        let cuts = |c: &[(EdgeId, f64, bool)], t: f64| {
            c.iter()
                .scan(0.0, |at, (_, l, _)| {
                    *at += l;
                    Some(*at / t)
                })
                .collect::<Vec<f64>>()
        };
        let mut breaks: Vec<f64> = cuts(&a, ta).into_iter().chain(cuts(&b, tb)).collect();
        breaks.push(0.0);
        breaks.sort_by(f64::total_cmp);
        breaks.dedup_by(|x, y| (*x - *y).abs() < 1e-9);
        // The stretch of chain `c` between shares `u0` and `u1`, walked up.
        let stretch_of = |c: &[(EdgeId, f64, bool)], t: f64, u0: f64, u1: f64| {
            let mid = (u0 + u1) / 2.0 * t;
            let mut start = 0.0;
            for (e, l, up) in c {
                if mid <= start + l {
                    let (d0, d1) = (u0 * t - start, u1 * t - start);
                    let (d0, d1) = (d0.clamp(0.0, *l), d1.clamp(0.0, *l));
                    return Some(if *up {
                        stretch(*e, Some(d0), Some(d1), false)
                    } else {
                        stretch(*e, Some(l - d1), Some(l - d0), true)
                    });
                }
                start += l;
            }
            None
        };
        for w in breaks.windows(2) {
            let &[u0, u1] = w else {
                continue;
            };
            if u1 - u0 < 1e-9 {
                continue;
            }
            let (Some(sa), Some(sb)) = (stretch_of(&a, ta, u0, u1), stretch_of(&b, tb, u0, u1))
            else {
                return Err(internal("a stretch outside its chain".into()));
            };
            self.sew_sides(sa, sb)?;
        }
        Ok(())
    }
}

fn p(x: f64, y: f64) -> DVec2 {
    DVec2::new(x, y)
}

/// A whole internal line as a seam side.
fn line_side(line: InternalLineId) -> SeamSideSpec {
    SeamSideSpec {
        line: Some(line),
        ..SeamSideSpec::default()
    }
}

/// A stretch of `edge`, from `from` to `to` mm along it (its ends when
/// omitted), walked backwards if `reversed`.
fn stretch(edge: EdgeId, from: Option<f64>, to: Option<f64>, reversed: bool) -> SeamSideSpec {
    SeamSideSpec {
        edge: Some(edge),
        from_mm: from,
        to_mm: to,
        reversed: Some(reversed),
        ..SeamSideSpec::default()
    }
}

/// Draft `template` into the project. Returns the created panels and
/// seams. In a project with a size range, the pieces are drafted in every
/// size and their points graded to match.
pub fn draft(
    tx: &mut Transaction<'_>,
    template: Template,
    sizes: Sizes,
    progress: &Progress,
) -> Result<Vec<AnyId>, CommandError> {
    let drafts = match tx.project().grading() {
        Some(_) => Some(every_size(tx.project(), template, sizes, progress)?),
        None => None,
    };
    // Each piece takes the room of its widest size on the board, so the
    // sizes nest without running into the next piece.
    let mut room: Vec<(f64, f64)> = Vec::new();
    for d in drafts.iter().flatten() {
        for (k, (id, origin)) in d.origins.iter().enumerate() {
            let Some(b) = d.project.panel_bounding_box(*id) else {
                continue;
            };
            let (lo, hi) = (b.min.x - origin, b.max.x - origin);
            match room.get_mut(k) {
                Some(r) => *r = (r.0.min(lo), r.1.max(hi)),
                None => room.push((lo, hi)),
            }
        }
    }
    let (created, origins) = draft_in(tx, template, sizes, &room)?;
    // The garment remembers its template, to be drafted again in sizes
    // set later.
    let garment = origins
        .keys()
        .next()
        .and_then(|p| tx.project().panel(*p))
        .map(|p| p.garment);
    if let Some(id) = garment
        && let Some(g) = tx.project().garments().get(&id).cloned()
    {
        tx.put_garment(
            id,
            Garment {
                template: Some(template_ref(template, sizes)),
                ..g
            },
        );
    }
    if let (Some(drafts), Some(g)) = (drafts, tx.project().grading().cloned()) {
        let panels: Vec<PanelId> = origins.keys().copied().collect();
        grade(tx, &panels, &drafts, g.base);
    }
    Ok(created)
}

/// How a garment remembers its template: its id and the sizes given.
fn template_ref(template: Template, sizes: Sizes) -> TemplateRef {
    let sizes = serde_json::to_value(sizes)
        .ok()
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or_default();
    TemplateRef {
        name: template.id().to_owned(),
        sizes,
    }
}

/// Grade the pieces of `garment`, drafted from a template and not graded
/// yet, by drafting that template for every size of the project's range.
/// The garment keeps the pieces' shapes as edited; each size moves them
/// as the template moves between sizes. Left as it is when a piece was
/// added or deleted; a piece whose outline gained or lost points is
/// skipped. Returns whether anything was graded.
pub fn grade_garment(
    tx: &mut Transaction<'_>,
    garment: GarmentId,
    progress: &Progress,
) -> Result<bool, CommandError> {
    let p = tx.project();
    let (Some(g), Some(reference)) = (
        p.grading().cloned(),
        p.garments().get(&garment).and_then(|g| g.template.clone()),
    ) else {
        return Ok(false);
    };
    let panels = garment_panels(p, garment);
    let graded = panels.iter().any(|id| {
        p.points()
            .values()
            .any(|pt| pt.panel == *id && !pt.grade.is_empty())
    });
    let Some((template, sizes)) = parse_ref(&reference) else {
        return Ok(false);
    };
    if graded || panels.is_empty() {
        return Ok(false);
    }
    let drafts = every_size(p, template, sizes, progress)?;
    // A piece added or deleted since: the pieces cannot be matched.
    if drafts.iter().any(|d| d.origins.len() != panels.len()) {
        return Ok(false);
    }
    Ok(grade(tx, &panels, &drafts, g.base))
}

/// The template and sizes a garment remembers.
fn parse_ref(reference: &TemplateRef) -> Option<(Template, Sizes)> {
    let template =
        serde_json::from_value::<Template>(serde_json::Value::String(reference.name.clone()));
    let sizes = serde_json::to_value(&reference.sizes).and_then(serde_json::from_value::<Sizes>);
    Some((template.ok()?, sizes.ok()?))
}

/// The panels of `garment`, in drafting order (the order of their IDs).
fn garment_panels(p: &Project, garment: GarmentId) -> Vec<PanelId> {
    p.panels()
        .iter()
        .filter(|(_, pn)| pn.garment == garment)
        .map(|(id, _)| *id)
        .collect()
}

/// Draft the pieces of `garment`, drafted from a template for `from`,
/// again for the project's avatar: each point and curve handle moves as
/// the template moves it between the two bodies, so the pieces keep what
/// was done to them; internal lines, pins and buttons follow. The body
/// measurements the template was given change as the body does; lengths
/// and ease stay. Returns the panels left as they were (their outline
/// changed since); all of them when pieces were added or removed.
pub(crate) fn redraft(
    tx: &mut Transaction<'_>,
    garment: GarmentId,
    from: &Avatar,
) -> Result<Vec<PanelId>, CommandError> {
    let p = tx.project();
    let Some(g) = p.garments().get(&garment).cloned() else {
        return Err(CommandError::not_found(garment));
    };
    let Some((template, sizes)) = g.template.as_ref().and_then(parse_ref) else {
        return Ok(Vec::new());
    };
    let to = avatar_of(p);
    let panels = garment_panels(p, garment);
    let new_sizes = Sizes {
        chest_cm: sizes
            .chest_cm
            .map(|v| v + to.body.bust_cm - from.body.bust_cm),
        waist_cm: sizes
            .waist_cm
            .map(|v| v + to.body.waist_cm - from.body.waist_cm),
        hips_cm: sizes
            .hips_cm
            .map(|v| v + to.body.hips_cm - from.body.hips_cm),
        ..sizes
    };
    let old = draft_for(from.clone(), template, sizes)?;
    let new = draft_for(to, template, new_sizes)?;
    if old.origins.len() != panels.len() || new.origins.len() != panels.len() {
        return Ok(panels);
    }
    let mut skipped = Vec::new();
    for (k, panel) in panels.iter().enumerate() {
        let moved = match (old.contour(k), new.contour(k)) {
            (Some(a), Some(b)) => crate::sources::move_piece(tx, *panel, &a, &b),
            _ => false,
        };
        if !moved {
            skipped.push(*panel);
        }
    }
    tx.put_garment(
        garment,
        Garment {
            template: Some(template_ref(template, new_sizes)),
            ..g
        },
    );
    Ok(skipped)
}

/// A template drafted in one size, on a project of its own.
struct Drafted {
    /// Where each piece's draft x = 0 lies on the board (mm), in drafting
    /// order.
    origins: BTreeMap<PanelId, f64>,
    project: Project,
}

/// `template` drafted for the body of every size of `p`'s range, in
/// parallel. Body measurements given (chest, waist, hips) were taken in
/// the base size: each size changes them as its body differs; lengths
/// and ease stay as given. Each size fits a body (seconds): `progress`
/// hears of each one drafted, and stops the drafting when it asks.
fn every_size(
    p: &Project,
    template: Template,
    sizes: Sizes,
    progress: &Progress,
) -> Result<Vec<Drafted>, CommandError> {
    let n = p.grading().map_or(0, |g| g.sizes.len());
    let done = AtomicU32::new(0);
    let total = u32::try_from(n).unwrap_or(u32::MAX);
    progress.report(&Status {
        task: "template.sizes",
        done: 0,
        total,
        item: String::new(),
    });
    (0..n)
        .into_par_iter()
        .map(|i| {
            // Sizes run side by side: stop before a size and after it
            // (a body fit cannot stop halfway).
            progress.check()?;
            let drafted = draft_size(p, template, sizes, i)?;
            progress.check()?;
            progress.report(&Status {
                task: "template.sizes",
                done: done.fetch_add(1, Ordering::Relaxed) + 1,
                total,
                item: p
                    .grading()
                    .and_then(|g| g.sizes.get(i))
                    .map(|s| s.name.clone())
                    .unwrap_or_default(),
            });
            Ok(drafted)
        })
        .collect()
}

impl Drafted {
    /// Piece `k`'s outline (each edge's start and handles) from its
    /// draft origin.
    fn contour(&self, k: usize) -> Option<Vec<crate::sources::ContourEdge>> {
        let (id, origin) = self.origins.iter().nth(k)?;
        let o = DVec2::new(*origin, 0.0);
        self.project
            .panel(*id)?
            .contour
            .iter()
            .map(|e| {
                let edge = self.project.edge(*e)?;
                let start = self.project.point(edge.start)?.pos - o;
                let handles = match edge.curve {
                    EdgeCurve::Cubic { c1, c2 } => Some([c1 - o, c2 - o]),
                    EdgeCurve::Line => None,
                };
                Some((start, handles))
            })
            .collect()
    }
}

/// `template` drafted for `avatar`, on a project of its own.
fn draft_for(avatar: Avatar, template: Template, sizes: Sizes) -> Result<Drafted, CommandError> {
    let mut project = Project::new("");
    let mut tx = Transaction::begin(&mut project);
    tx.set_avatar(Some(avatar));
    let (_, origins) = draft_in(&mut tx, template, sizes, &[])?;
    Ok(Drafted {
        origins,
        project: tx.project().clone(),
    })
}

/// `template` drafted for the body of size `i` of `p`'s range.
fn draft_size(
    p: &Project,
    template: Template,
    sizes: Sizes,
    i: usize,
) -> Result<Drafted, CommandError> {
    let avatar = size_avatar(p, i).ok_or_else(|| internal(format!("no size {i}")))?;
    let size = p
        .grading()
        .and_then(|g| g.sizes.get(i))
        .ok_or_else(|| internal(format!("no size {i}")))?;
    let sizes = Sizes {
        chest_cm: sizes.chest_cm.map(|v| v + size.bust_cm),
        waist_cm: sizes.waist_cm.map(|v| v + size.waist_cm),
        hips_cm: sizes.hips_cm.map(|v| v + size.hips_cm),
        ..sizes
    };
    draft_for(avatar, template, sizes)
}

/// The avatar a template is drafted for in size `i`: the project's (or a
/// standard one) changed by the size's differences, its typed
/// measurements graded like their estimates.
fn size_avatar(p: &Project, i: usize) -> Option<Avatar> {
    let avatar = avatar_of(p);
    let size = p.grading()?.sizes.get(i)?;
    let body = Body {
        height_cm: avatar.body.height_cm + size.height_cm,
        bust_cm: avatar.body.bust_cm + size.bust_cm,
        waist_cm: avatar.body.waist_cm + size.waist_cm,
        hips_cm: avatar.body.hips_cm + size.hips_cm,
        age_years: avatar.body.aged(size.age_years),
        ..avatar.body
    };
    Some(Avatar {
        measures: r3d_core::body::regraded(&avatar.measures, &avatar.body, &body),
        body,
        preset: None,
        mesh: None,
    })
}

/// One drafted piece: its contour edges and its draft origin on the board.
type DraftedPiece = (Vec<EdgeId>, DVec2);

/// Grade `panels` (a template's pieces, in drafting order) from its
/// drafts in every size: each contour point (and curve handle) moves as
/// it does between the base size's draft and that size's, measured from
/// the piece's draft origin (on most pieces a centre line), so the sizes
/// nest. Pieces whose outline does not match the draft's are skipped.
/// Returns whether any piece was graded.
fn grade(tx: &mut Transaction<'_>, panels: &[PanelId], drafts: &[Drafted], base: usize) -> bool {
    let n = drafts.len();
    let piece = |d: &Drafted, k: usize| -> Option<DraftedPiece> {
        let (id, origin) = d.origins.iter().nth(k)?;
        Some((
            d.project.panel(*id)?.contour.clone(),
            DVec2::new(*origin, 0.0),
        ))
    };
    let mut any = false;
    for (k, panel) in panels.iter().enumerate() {
        let Some(own) = tx.project().panel(*panel).map(|p| p.contour.clone()) else {
            continue;
        };
        let Some(drafted) = drafts
            .iter()
            .map(|d| piece(d, k))
            .collect::<Option<Vec<DraftedPiece>>>()
        else {
            continue;
        };
        if drafted.iter().any(|(c, _)| c.len() != own.len()) {
            continue;
        }
        // Edge `j` of the piece in size `i`, its start point and the
        // piece's origin.
        let at = |i: usize, j: usize| -> Option<(&Edge, DVec2, DVec2)> {
            let (c, origin) = drafted.get(i)?;
            let project = &drafts.get(i)?.project;
            let edge = project.edge(*c.get(j)?)?;
            Some((edge, project.point(edge.start)?.pos, *origin))
        };
        for (j, edge_id) in own.iter().enumerate() {
            let Some(edge) = tx.project().edge(*edge_id).cloned() else {
                continue;
            };
            let Some((base_edge, base_start, base_origin)) = at(base, j) else {
                continue;
            };
            let mut grade = vec![DVec2::ZERO; n];
            let mut handles = vec![[DVec2::ZERO; 2]; n];
            for i in (0..n).filter(|i| *i != base) {
                let Some((e, start, origin)) = at(i, j) else {
                    continue;
                };
                if let Some(slot) = grade.get_mut(i) {
                    *slot = (start - origin) - (base_start - base_origin);
                }
                if let (
                    EdgeCurve::Cubic { c1, c2 },
                    EdgeCurve::Cubic { c1: b1, c2: b2 },
                    Some(slot),
                ) = (&e.curve, &base_edge.curve, handles.get_mut(i))
                {
                    *slot = [
                        (*c1 - origin) - (*b1 - base_origin),
                        (*c2 - origin) - (*b2 - base_origin),
                    ];
                }
            }
            if let Some(point) = tx.project().point(edge.start).cloned()
                && grade.iter().any(|g| *g != DVec2::ZERO)
            {
                tx.put_point(edge.start, Point { grade, ..point });
                any = true;
            }
            if matches!(edge.curve, EdgeCurve::Cubic { .. })
                && handles.iter().flatten().any(|g| *g != DVec2::ZERO)
            {
                tx.put_edge(
                    *edge_id,
                    Edge {
                        grade: handles,
                        ..edge
                    },
                );
            }
        }
    }
    any
}

/// Draft `template` in the avatar's body, ungraded; piece `k` takes at
/// least `room[k]` on the board (its extent around its origin, mm).
fn draft_in(
    tx: &mut Transaction<'_>,
    template: Template,
    sizes: Sizes,
    room: &[(f64, f64)],
) -> Result<(Vec<AnyId>, BTreeMap<PanelId, f64>), CommandError> {
    let avatar = avatar_of(tx.project());
    let sizes = sizes.resolve(template, &avatar, waist_height_cm(tx.project())?);
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
        origins: BTreeMap::new(),
        room,
    };
    match template {
        Template::StraightSkirt => skirt(&mut d, &sizes, &avatar.body, 0.0)?,
        Template::ALineSkirt => skirt(&mut d, &sizes, &avatar.body, 0.2)?,
        Template::TShirt => {
            t_shirt(&mut d, &sizes, &avatar, Top::TShirt)?;
        }
        Template::Hoodie => {
            t_shirt(&mut d, &sizes, &avatar, Top::Hoodie)?;
        }
        Template::Bodysuit => {
            t_shirt(&mut d, &sizes, &avatar, Top::Bodysuit)?;
        }
        Template::Sweatshirt => {
            t_shirt(&mut d, &sizes, &avatar, Top::Sweatshirt)?;
        }
        Template::PyjamaTop => {
            t_shirt(&mut d, &sizes, &avatar, Top::Pyjama)?;
        }
        Template::PyjamaBottoms => trousers(&mut d, &sizes, &avatar, Legs::Pyjama)?,
        Template::Pyjama | Template::JoggingSuit => {
            return Err(internal("a set is drafted part by part".into()));
        }
        Template::Shirt => shirt(&mut d, &sizes, &avatar.body)?,
        Template::Trousers => trousers(&mut d, &sizes, &avatar, Legs::Trousers)?,
        Template::Joggers => trousers(&mut d, &sizes, &avatar, Legs::Joggers)?,
        Template::Jeans => trousers(&mut d, &sizes, &avatar, Legs::Jeans)?,
        Template::Shorts => trousers(&mut d, &sizes, &avatar, Legs::Shorts)?,
        Template::Dress => dress(&mut d, &sizes, &avatar.body)?,
        Template::ToteBag => tote_bag(&mut d, &sizes)?,
    }
    let fabric = match template {
        Template::TShirt
        | Template::Hoodie
        | Template::Joggers
        | Template::Sweatshirt
        | Template::Bodysuit
        | Template::PyjamaTop
        | Template::PyjamaBottoms => Fabric::Jersey,
        Template::ToteBag | Template::Trousers | Template::Jeans => Fabric::Denim,
        _ => Fabric::Cotton,
    };
    let title = template.title().to_owned();
    if fresh {
        // The project's garment becomes this one.
        let garment = d.tx.project().default_garment();
        GarmentSetFabric { garment, fabric }.apply(d.tx)?;
        if let Some(garment) = garment {
            GarmentRename {
                garment,
                name: title,
            }
            .apply(d.tx)?;
        }
    } else {
        // A garment of its own, worn over the others.
        let out = GarmentAdd {
            name: title,
            fabric,
            color: None,
        }
        .apply(d.tx)?;
        if let Some(AnyId::Garment(garment)) = out.created.first().copied() {
            let pieces: Vec<PanelId> = d
                .created
                .iter()
                .filter_map(|a| match a {
                    AnyId::Panel(p) => Some(*p),
                    _ => None,
                })
                .collect();
            for panel in pieces {
                PanelSetGarment { panel, garment }.apply(d.tx)?;
            }
            d.created.insert(0, AnyId::Garment(garment));
        }
    }
    Ok((d.created, d.origins))
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
    // The backs start with their centre-back edges side by side at 180°,
    // where the seam spans the gap between the legs: a gap left there at
    // the start (backs placed by their middles at 120° and 240°, as if the
    // pieces were equal) was closed straight across, between the legs, and
    // the back caught there in a fold. Gaps at the side seams close over
    // the hips instead.
    let bbox = |piece: &Piece| d.tx.project().panel_bounding_box(piece.id);
    let (Some(left_box), Some(right_box)) = (bbox(&back), bbox(&right)) else {
        return Err(internal("a skirt piece has no outline".into()));
    };
    let (left_cb, right_cb) = (left_box.max.x, right_box.min.x);
    d.place_at(&back, Around::Body, 180.0, top, 30.0, Some(left_cb))?;
    d.place_at(&right, Around::Body, 180.0, top, 30.0, Some(right_cb))?;
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

/// The garments drafted on the T-shirt's block.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Top {
    TShirt,
    Hoodie,
    /// A jogging suit's top: a hoodie without its hood.
    Sweatshirt,
    /// A pyjama's top: long sleeves gathered at the wrists, a loose hem.
    Pyjama,
    /// A baby's bodysuit: the bodice goes down under the crotch.
    Bodysuit,
}

impl Top {
    fn long_sleeves(self) -> bool {
        matches!(self, Top::Hoodie | Top::Sweatshirt | Top::Pyjama)
    }

    /// Whether an elastic gathers the hem to the hips.
    fn banded(self) -> bool {
        matches!(self, Top::Hoodie | Top::Sweatshirt)
    }
}

/// A bodysuit's crotch, as a fraction of the hips (each side of the
/// centre line), and the height of its leg openings' sides above the
/// crotch, as a fraction of the waist's height above it.
const CROTCH_HALF: f64 = 1.0 / 14.0;
const LEG_SIDE: f64 = 0.45;

/// A T-shirt: front and back bodices (whole, symmetric) and two short
/// sleeves (the right one a mirror copy of the left). With long sleeves
/// (hoodie, sweatshirt, pyjama) they narrow towards the wrists, gathered
/// there by an elastic; a hoodie's and a sweatshirt's hem is gathered to
/// the hips, and a hoodie has a hood sewn to the neckline. A bodysuit's
/// bodices go on below the hips into leg openings that curve in to the
/// crotch, where the front and back are sewn together (a bodysuit's
/// snaps). The bodices' edges, from the bottom, counter-clockwise: the
/// hem (a bodysuit: the crotch, then the right leg opening), the right
/// side, armhole and shoulder, the neckline, the left shoulder, armhole
/// and side (a bodysuit: then the left leg opening). Returns the front
/// and back.
fn t_shirt(
    d: &mut Draft,
    s: &Sizes,
    avatar: &Avatar,
    top: Top,
) -> Result<(Piece, Piece), CommandError> {
    let body = &avatar.body;
    let chest = mm("chest_cm", s.chest_cm)?;
    let len = mm("length_cm", s.length_cm)?;
    let sleeve = mm("sleeve_cm", s.sleeve_cm)?;
    let ease = s.ease_cm.unwrap_or_default() * 10.0;
    // Half widths: the body from the chest, the hem from the hips (the
    // larger), shoulder and neck measured on the body.
    let w = (chest + ease) / 4.0;
    let hips = s.hips_cm.unwrap_or(body.hips_cm) * 10.0;
    let hem = w.max((hips + ease) / 4.0);
    let upper = Upper::of(d.tx.project())?;
    let shoulder = upper.shoulder_half(chest);
    let neck = upper.neck_half();
    let y_s = len - upper.shoulder_drop();
    let y_a = y_s - upper.armhole_depth();
    let h = body.height_cm;
    let stature = Stature::of(body);
    // A bodysuit's leg openings: from the crotch (its bottom, y = 0) up
    // to the side, a little below the hips.
    let crotch = CROTCH_HALF * hips;
    let rise = (waist_height_cm(d.tx.project())? - stature.crotch * h) * 10.0;
    let y_leg = (LEG_SIDE * rise).min(0.5 * y_a);
    let leg = [p(crotch, 0.6 * y_leg), p(0.6 * hem, y_leg)];
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
        let mut outline = if top == Top::Bodysuit {
            vec![
                (p(-crotch, 0.0), None),
                (p(crotch, 0.0), Some(leg)),
                (p(hem, y_leg), None),
            ]
        } else {
            vec![(p(-hem, 0.0), None), (p(hem, 0.0), None)]
        };
        let mirror = |[a, b]: [DVec2; 2]| [p(-b.x, b.y), p(-a.x, a.y)];
        outline.extend([
            (p(w, y_a), Some(hole(1.0))),
            (p(shoulder, y_s), None),
            (p(neck, len), Some([p(neck, len - k), p(-neck, len - k)])),
            (p(-neck, len), None),
            (p(-shoulder, y_s), Some(reverse(hole(-1.0)))),
            (p(-w, y_a), None),
        ]);
        if top == Top::Bodysuit {
            outline.push((p(-hem, y_leg), Some(mirror(leg))));
        }
        d.panel(name, &outline)
    };
    let front = bodice(d, "Front", neck + 10.0)?;
    let back = bodice(d, "Back", 20.0)?;
    // Edge `k` of a T-shirt's bodice is edge `k + o` of a bodysuit's.
    let o = usize::from(top == Top::Bodysuit);
    let hole_len = armhole_length(d, &front, &back, 2 + o)?;
    let half = (0.33 * chest + 60.0 * upper.scale) / 2.0;
    // A long sleeve narrows to the shirt's cuff.
    let sleeve_hem = if top.long_sleeves() {
        (0.12 * chest + 150.0 * upper.scale) / 2.0
    } else {
        half - 15.0 * upper.scale
    };
    let left = sleeve_piece(d, "Sleeve left", hole_len, half, sleeve_hem, sleeve)?;
    let right = d.mirror(&left, "Sleeve right")?;
    // Seen from outside, the front's +x side is the body's left, the
    // back's +x side its right; a left sleeve's +x side faces the back.
    d.sew(front.edge(1 + o)?, back.edge(7 + o)?)?;
    d.sew(front.edge(7 + o)?, back.edge(1 + o)?)?;
    d.sew(front.edge(3 + o)?, back.edge(5 + o)?)?;
    d.sew(front.edge(5 + o)?, back.edge(3 + o)?)?;
    d.sew(left.edge(3)?, front.edge(2 + o)?)?;
    d.sew(left.edge(2)?, back.edge(6 + o)?)?;
    d.sew(right.edge(3)?, front.edge(6 + o)?)?;
    d.sew(right.edge(2)?, back.edge(2 + o)?)?;
    d.sew(left.edge(1)?, left.edge(4)?)?;
    d.sew(right.edge(1)?, right.edge(4)?)?;
    if top == Top::Bodysuit {
        d.sew(front.edge(0)?, back.edge(0)?)?;
    }
    let top_cm = stature.neck * h + TOP_LIFT_CM;
    d.place(&front, Around::Body, 0.0, top_cm)?;
    d.place(&back, Around::Body, 180.0, top_cm)?;
    d.place(&left, Around::LeftArm, 0.0, stature.shoulder * h)?;
    d.place(&right, Around::RightArm, 0.0, stature.shoulder * h)?;
    if top.long_sleeves() {
        // The cuffs close to the wrist with room for the hand to slide
        // through.
        let wrist = (avatar.measure(Measure::Wrist).cm + 4.0) * 10.0;
        for sleeve in [&left, &right] {
            d.elastic(whole(sleeve.edge(0)?), wrist / (2.0 * sleeve_hem))?;
        }
    }
    if top.banded() {
        // The hem's band to the hips.
        let band = hips / (4.0 * hem);
        for piece in [&front, &back] {
            d.elastic(whole(piece.edge(0)?), band)?;
        }
    }
    if top == Top::Hoodie {
        hood(d, body, &front, &back)?;
    }
    Ok((front, back))
}

/// A whole edge, as a seam's side.
fn whole(edge: EdgeId) -> SeamSideSpec {
    SeamSideSpec {
        edge: Some(edge),
        ..SeamSideSpec::default()
    }
}

/// A hood's height (neck to the top, along its face opening's side) and
/// depth (face to the back of the head), as fractions of an adult's
/// stature; a child's follow its head, larger for its stature.
const HOOD_HEIGHT: f64 = 0.21;
const HOOD_DEPTH: f64 = 0.14;
/// A hood starts this far over the head's top (cm).
const HOOD_OVER_HEAD_CM: f64 = 2.0;

/// A hood in two halves sewn down the middle, over the head, and to the
/// neckline of `front` and `back` (their edge 4), the halves meeting at
/// the centre front.
fn hood(d: &mut Draft, body: &Body, front: &Piece, back: &Piece) -> Result<(), CommandError> {
    let len = |d: &Draft, e: EdgeId| d.tx.project().edge_length(e).unwrap_or_default();
    // Each half takes half of each neckline.
    let (f, b) = (len(d, front.edge(4)?) / 2.0, len(d, back.edge(4)?) / 2.0);
    // An adult's head, from the neck up, is 1 − NECK_HEIGHT of its
    // stature.
    let head = (1.0 - Stature::of(body).neck) / (1.0 - NECK_HEIGHT);
    let h = body.height_cm * 10.0 * head;
    let height = HOOD_HEIGHT * h;
    let depth = (HOOD_DEPTH * h).max(f + b + 30.0);
    // The body's left half, seen from outside, the face on its left:
    // edges 0 front neckline, 1 back neckline, 2 the middle seam, from
    // the back of the neck over the head, 3 the face opening.
    let left = d.panel(
        "Hood left",
        &[
            (p(0.0, 0.0), None),
            (p(f, 0.0), None),
            (
                p(f + b, 0.0),
                Some([
                    p(depth + 20.0, 0.3 * height),
                    p(0.75 * depth, height + 10.0),
                ]),
            ),
            (p(0.25 * depth, height), None),
        ],
    )?;
    let right = d.mirror(&left, "Hood right")?;
    d.sew(left.edge(2)?, right.edge(2)?)?;
    // The front's neckline runs from the body's left over the centre
    // front, the back's from the body's right over the centre back.
    d.sew_sides(
        whole(left.edge(0)?),
        stretch(front.edge(4)?, None, Some(f), true),
    )?;
    d.sew_sides(
        whole(right.edge(0)?),
        stretch(front.edge(4)?, Some(f), None, true),
    )?;
    d.sew_sides(
        whole(left.edge(1)?),
        stretch(back.edge(4)?, Some(b), None, true),
    )?;
    d.sew_sides(
        whole(right.edge(1)?),
        stretch(back.edge(4)?, None, Some(b), true),
    )?;
    // Its top just over the head, the middle seam at the back (as a
    // skirt's backs): it settles over the head and falls down the back.
    // Started its height above the neck, 11 cm above the head, it
    // crumpled at the nape.
    let bbox = |piece: &Piece| d.tx.project().panel_bounding_box(piece.id);
    let (Some(left_box), Some(right_box)) = (bbox(&left), bbox(&right)) else {
        return Err(internal("a hood piece has no outline".into()));
    };
    let (left_back, right_back) = (left_box.max.x, right_box.min.x);
    let hood_top = body.height_cm + HOOD_OVER_HEAD_CM;
    d.place_at(&left, Around::Body, 180.0, hood_top, 30.0, Some(left_back))?;
    d.place_at(
        &right,
        Around::Body,
        180.0,
        hood_top,
        30.0,
        Some(right_back),
    )?;
    Ok(())
}

/// Where along `curve`, rising from its start, it reaches the height `y`
/// (bisection; its end when it never does).
fn rising_to(curve: &r3d_geom::CubicBez, y: f64) -> f64 {
    let (mut lo, mut hi) = (0.0, 1.0);
    for _ in 0..50 {
        let mid = (lo + hi) / 2.0;
        if curve.eval(mid).y < y {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    (lo + hi) / 2.0
}

/// The longer armhole of a front and a back (their edge `k`).
fn armhole_length(d: &Draft, front: &Piece, back: &Piece, k: usize) -> Result<f64, CommandError> {
    let len = |piece: &Piece| -> Result<f64, CommandError> {
        Ok(d.tx
            .project()
            .edge_length(piece.edge(k)?)
            .unwrap_or_default())
    };
    Ok(len(front)?.max(len(back)?))
}

/// The least underarm seam a sleeve keeps below its cap (mm).
const UNDERARM_MIN_MM: f64 = 30.0;

/// A sleeve `height` high (mm), `half` its half width at the underarm and
/// `hem` at the hem, its cap as long as the armhole (`hole_len`, each
/// side): the cap's height is found by bisection. A sleeve too short for
/// its cap and [`UNDERARM_MIN_MM`] below it (a baby's short sleeve) is
/// lengthened. Edges: 0 hem, 1 right
/// underarm seam, 2 right cap (towards the back on the left arm), 3 left
/// cap, 4 left underarm seam.
fn sleeve_piece(
    d: &mut Draft,
    name: &str,
    hole_len: f64,
    half: f64,
    hem: f64,
    sleeve: f64,
) -> Result<Piece, CommandError> {
    // The cap `h` high, its top at `sleeve`.
    let cap = |h: f64, sleeve: f64| {
        let top = p(0.0, sleeve);
        let under = p(half, sleeve - h);
        [
            under,
            p(0.6 * half, sleeve - 0.35 * h),
            p(0.35 * half, sleeve),
            top,
        ]
    };
    let long_enough = |h: f64| {
        let [a, b, c, e] = cap(h, sleeve);
        CubicBez::new(a, b, c, e).arc_length(0.01) >= hole_len
    };
    // A cap as high as the armhole is long is longer than it.
    let highest = if long_enough(sleeve * 0.9) {
        sleeve * 0.9
    } else {
        hole_len
    };
    let (mut lo, mut hi) = (10.0, highest);
    for _ in 0..40 {
        let mid = (lo + hi) / 2.0;
        let [a, b, c, e] = cap(mid, sleeve);
        if CubicBez::new(a, b, c, e).arc_length(0.01) < hole_len {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    let sleeve = sleeve.max(lo + UNDERARM_MIN_MM);
    let [under, c1, c2, top] = cap(lo, sleeve);
    let mirror_x = |q: DVec2| p(-q.x, q.y);
    d.panel(
        name,
        &[
            (p(-hem, 0.0), None),
            (p(hem, 0.0), None),
            (under, Some([c1, c2])),
            (top, Some([mirror_x(c2), mirror_x(c1)])),
            (mirror_x(under), None),
        ],
    )
}

/// The neckline curve of a front half, from the neck point `(neck, top)`
/// down to the centre front, `depth` below it.
fn quarter_neck(neck: f64, top: f64, depth: f64) -> [DVec2; 2] {
    [p(neck, top - 0.55 * depth), p(0.55 * neck, top - depth)]
}

/// Collar and cuff heights (mm).
const COLLAR_MM: f64 = 35.0;
/// How far a shirt front's band reaches past the centre front (mm): the
/// fronts lap over each other by twice this.
const BAND_MM: f64 = 15.0;
/// How much longer a cuff is than the wrist it closes around (mm).
const CUFF_LAP_MM: f64 = 20.0;
/// The shirt's buttons (mm).
const BUTTON_MM: f64 = 11.0;
/// A trouser fly's zipper (mm), at most 60 % of the centre front.
const FLY_MM: f64 = 180.0;
const CUFF_MM: f64 = 60.0;
/// A shirt's yoke reaches this far below the shoulders' tops (mm).
const YOKE_MM: f64 = 70.0;

/// A shirt: two fronts buttoned at the centre front, a whole back, long
/// sleeves ending in buttoned cuffs, and a stand collar around the
/// neckline. The body's block is the T-shirt's, longer and looser. Each
/// front has a band past the centre front; buttons on one front's centre
/// line, buttonholes on the other's, which laps over it (the left front
/// over the right on a men's body, the right over the left otherwise).
fn shirt(d: &mut Draft, s: &Sizes, body: &Body) -> Result<(), CommandError> {
    let chest = mm("chest_cm", s.chest_cm)?;
    let len = mm("length_cm", s.length_cm)?;
    let sleeve = mm("sleeve_cm", s.sleeve_cm)? - CUFF_MM;
    positive("sleeve_cm", sleeve)?;
    let ease = s.ease_cm.unwrap_or_default() * 10.0;
    let w = (chest + ease) / 4.0;
    let upper = Upper::of(d.tx.project())?;
    let shoulder = upper.shoulder_half(chest);
    let neck = upper.neck_half();
    let y_s = len - upper.shoulder_drop();
    let y_a = y_s - upper.armhole_depth();
    // The hem takes the hips with the same ease, the side seam flaring
    // from the armhole: drafted on the chest alone, a women's 52 shirt
    // (hips 8 cm over the bust) opened at its front over the hips.
    let hem = w.max((body.hips_cm * 10.0 + ease) / 4.0);
    let hole = |sign: f64| {
        [
            p(sign * (shoulder + 0.3 * (w - shoulder)), y_a),
            p(sign * shoulder, y_a + 0.45 * (y_s - y_a)),
        ]
    };
    // The body's left front, centre front at x = 0, its band past it.
    // Edges: 0 hem, 1 side, 2 armhole, 3 shoulder, 4 neckline, 5 the
    // band's top, 6 the front edge.
    let neck_depth = neck + 10.0;
    let front = d.panel(
        "Front left",
        &[
            (p(-BAND_MM, 0.0), None),
            (p(hem, 0.0), None),
            (p(w, y_a), Some(hole(1.0))),
            (p(shoulder, y_s), None),
            (p(neck, len), Some(quarter_neck(neck, len, neck_depth))),
            (p(0.0, len - neck_depth), None),
            (p(-BAND_MM, len - neck_depth), None),
        ],
    )?;
    let front_right = d.mirror(&front, "Front right")?;
    // The back in two: a yoke across the shoulders, sewn YOKE_MM below
    // them, and the back below it. Each armhole is cut where the yoke's
    // seam meets it (de Casteljau).
    let y_k = y_s - YOKE_MM;
    let [c1, c2] = hole(1.0);
    let armhole = r3d_geom::CubicBez::new(p(w, y_a), c1, c2, p(shoulder, y_s));
    let (below, above) = armhole.split(rising_to(&armhole, y_k));
    let x_k = below.p3.x;
    let mirror = |q: DVec2| p(-q.x, q.y);
    // The back: 0 hem, 1 right side, 2 right armhole, 3 the yoke seam,
    // 4 left armhole, 5 left side.
    let back = d.panel(
        "Back",
        &[
            (p(-hem, 0.0), None),
            (p(hem, 0.0), None),
            (p(w, y_a), Some([below.p1, below.p2])),
            (p(x_k, y_k), None),
            (p(-x_k, y_k), Some([mirror(below.p2), mirror(below.p1)])),
            (p(-w, y_a), None),
        ],
    )?;
    // The yoke: 0 its seam to the back, 1 right armhole, 2 right
    // shoulder, 3 neckline, 4 left shoulder, 5 left armhole.
    let k = 4.0 / 3.0 * 20.0;
    let yoke = d.panel(
        "Yoke",
        &[
            (p(-x_k, y_k), None),
            (p(x_k, y_k), Some([above.p1, above.p2])),
            (p(shoulder, y_s), None),
            (p(neck, len), Some([p(neck, len - k), p(-neck, len - k)])),
            (p(-neck, len), None),
            (
                p(-shoulder, y_s),
                Some([mirror(above.p2), mirror(above.p1)]),
            ),
        ],
    )?;
    let edge_len = |e: EdgeId| d.tx.project().edge_length(e).unwrap_or_default();
    // The back's armhole, below the yoke then on it.
    let (hole_low, hole_high) = (edge_len(back.edge(2)?), edge_len(yoke.edge(1)?));
    let hole_len = edge_len(front.edge(2)?).max(hole_low + hole_high);
    let cuff = 0.12 * chest + 150.0 * upper.scale;
    let half = (0.33 * chest + 70.0 * upper.scale) / 2.0;
    let mut left = sleeve_piece(d, "Sleeve left", hole_len, half, cuff / 2.0, sleeve)?;
    // Its back cap in two, where the yoke meets the armhole: edges 0 hem,
    // 1 right underarm seam, 2 back cap below the yoke, 3 on it, 4 front
    // cap, 5 left underarm seam.
    let cap =
        d.tx.project()
            .edge_length(left.edge(2)?)
            .unwrap_or_default();
    let out = EdgeSplit {
        edge: left.edge(2)?,
        at_mm: cap * hole_low / (hole_low + hole_high),
    }
    .apply(d.tx)?;
    let upper = out
        .created
        .iter()
        .find_map(|id| match id {
            AnyId::Edge(e) => Some(*e),
            _ => None,
        })
        .ok_or_else(|| internal("the sleeve cap was not split".into()))?;
    left.edges.insert(3, upper);
    let right = d.mirror(&left, "Sleeve right")?;
    // Cuffs, with a lap to button: edges 0 wrist, 1 right end, 2 top
    // (sewn to the sleeve but for the lap), 3 left end.
    let cuff_left = d.panel(
        "Cuff left",
        &[
            (p(0.0, 0.0), None),
            (p(cuff + CUFF_LAP_MM, 0.0), None),
            (p(cuff + CUFF_LAP_MM, CUFF_MM), None),
            (p(0.0, CUFF_MM), None),
        ],
    )?;
    let cuff_right = d.mirror(&cuff_left, "Cuff right")?;
    // The collar runs from the left centre front around the back to the
    // right one (seen from outside, centred at the back): edges 0–2 its
    // neck edge (left front, back, right front), 3 right end, 4 top, 5
    // left end.
    let edge_len = |e: EdgeId| d.tx.project().edge_length(e).unwrap_or_default();
    let (f, b) = (edge_len(front.edge(4)?), edge_len(yoke.edge(3)?));
    let collar = d.panel(
        "Collar",
        &[
            (p(0.0, 0.0), None),
            (p(f, 0.0), None),
            (p(f + b, 0.0), None),
            (p(2.0 * f + b, 0.0), None),
            (p(2.0 * f + b, COLLAR_MM), None),
            (p(0.0, COLLAR_MM), None),
        ],
    )?;
    d.sew(front.edge(1)?, back.edge(5)?)?;
    d.sew(front_right.edge(1)?, back.edge(1)?)?;
    d.sew(front.edge(3)?, yoke.edge(4)?)?;
    d.sew(front_right.edge(3)?, yoke.edge(2)?)?;
    d.sew(back.edge(3)?, yoke.edge(0)?)?;
    d.sew(left.edge(4)?, front.edge(2)?)?;
    d.sew(right.edge(4)?, front_right.edge(2)?)?;
    // Each sleeve's back cap: below the yoke to the back, then to the yoke.
    d.sew(left.edge(2)?, back.edge(4)?)?;
    d.sew(left.edge(3)?, yoke.edge(5)?)?;
    d.sew(right.edge(2)?, back.edge(2)?)?;
    d.sew(right.edge(3)?, yoke.edge(1)?)?;
    d.sew(left.edge(1)?, left.edge(5)?)?;
    d.sew(right.edge(1)?, right.edge(5)?)?;
    // Each cuff's top, but the lap, to its sleeve's wrist; one button
    // closes the cuff around the wrist, the lap over its other end.
    for (c, s) in [(&cuff_left, &left), (&cuff_right, &right)] {
        d.sew_sides(
            stretch(c.edge(2)?, Some(CUFF_LAP_MM), None, false),
            stretch(s.edge(0)?, None, None, true),
        )?;
        let y = CUFF_MM / 2.0;
        d.button(
            c,
            p(CUFF_LAP_MM / 2.0, y),
            c,
            p(cuff + CUFF_LAP_MM / 2.0, y),
        )?;
    }
    // The fronts: buttons down the centre line of the under front,
    // buttonholes on the one lapped over it, from below the collar to
    // a tenth of the length above the hem, about 9 cm apart.
    let (under, over) = if body.masculinity >= 0.5 {
        (&front_right, &front)
    } else {
        (&front, &front_right)
    };
    let (y_top, y_low) = (len - neck_depth - 20.0, 0.1 * len);
    let count = ((y_top - y_low) / 90.0).round().max(1.0) as usize + 1;
    for k in 0..count {
        let y = y_top - (y_top - y_low) * k as f64 / (count - 1) as f64;
        d.button(under, p(0.0, y), over, p(0.0, y))?;
    }
    d.sew(collar.edge(0)?, front.edge(4)?)?;
    d.sew(collar.edge(1)?, yoke.edge(3)?)?;
    d.sew(collar.edge(2)?, front_right.edge(4)?)?;
    let h = body.height_cm;
    let stature = Stature::of(body);
    let top = stature.neck * h + TOP_LIFT_CM;
    // The front that laps over starts a little further out.
    for f in [&front, &front_right] {
        let offset = if f.id == over.id { 38.0 } else { 30.0 };
        d.place_at(f, Around::Body, 0.0, top, offset, Some(f.origin_x))?;
    }
    d.place(&yoke, Around::Body, 180.0, top)?;
    d.place(&back, Around::Body, 180.0, top - (len - y_k) / 10.0)?;
    // The collar's lower edge at the back neck, 2 cm below the shoulders'
    // tops.
    d.place(&collar, Around::Body, 180.0, top - 2.0 + COLLAR_MM / 10.0)?;
    let shoulder_top = stature.shoulder * h;
    d.place(&left, Around::LeftArm, 0.0, shoulder_top)?;
    d.place(&right, Around::RightArm, 0.0, shoulder_top)?;
    let wrist = shoulder_top - arm_drop(d.tx.project())? * sleeve / 10.0;
    d.place(&cuff_left, Around::LeftArm, 0.0, wrist)?;
    d.place(&cuff_right, Around::RightArm, 0.0, wrist)?;
    // Construction details (spec 31): the collar and the cuffs
    // interfaced; a yoke cut off the back below the shoulders and
    // topstitched along its seam.
    for piece in [&collar, &cuff_left, &cuff_right] {
        d.interface(piece.id)?;
    }
    d.topstitch(yoke.edge(0)?, 2, 1.5)?;
    Ok(())
}

/// The garments drafted on the trousers' block.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Legs {
    Trousers,
    Joggers,
    Jeans,
    Shorts,
    /// A pyjama's bottoms: the joggers without elastic at the ankles,
    /// looser, down to the trousers' length.
    Pyjama,
}

impl Legs {
    /// Pulled on: an elastic waist and no fly.
    fn pulled_on(self) -> bool {
        matches!(self, Legs::Joggers | Legs::Pyjama)
    }
}

/// Room around the thigh at a pair of shorts' hem (cm).
const SHORTS_HEM_EASE_CM: f64 = 10.0;

/// The trousers' block for a body: a front and a back per leg, drawn for
/// the left leg (the crease at x = 0, y up from the hem, mm).
struct Block {
    len: f64,
    rise: f64,
    y_c: f64,
    depth: f64,
    y_h: f64,
    hem_f: f64,
    hem_b: f64,
    ext_f: f64,
    ext_b: f64,
    out: f64,
    side_in: f64,
    cf: f64,
    cf_in: f64,
    cb: f64,
    cb_in: f64,
    cb_up: f64,
    crotch_f: DVec2,
    crotch_bx: f64,
    y_cb: f64,
    slant: f64,
    /// The body's waist height (cm).
    waist_top: f64,
}

impl Block {
    fn new(d: &Draft, s: &Sizes, avatar: &Avatar, legs: Legs) -> Result<Block, CommandError> {
        let body = &avatar.body;
        let waist = mm("waist_cm", s.waist_cm)?;
        let hips = mm("hips_cm", s.hips_cm)?;
        let len = mm("length_cm", s.length_cm)?;
        let ease = s.ease_cm.unwrap_or_default() * 10.0;
        let h = body.height_cm;
        let waist_top = waist_height_cm(d.tx.project())?;
        // The crotch line 1.5 cm below the body's, the hip line at the hips.
        let crotch = Stature::of(body).crotch * h;
        let rise = ((waist_top - crotch) * 10.0 + 15.0).clamp(150.0, 0.6 * len);
        let y_c = len - rise;
        let depth = (HIP_DEPTH * h * 10.0).min(0.8 * rise);
        let y_h = len - depth;
        // Widths at the hip line, crotch extensions, half hems.
        let front_hip = (hips + ease) / 4.0 - 10.0;
        let back_hip = (hips + ease) / 4.0 + 10.0;
        let (ext_f, ext_b) = (hips / 16.0, hips / 10.0 + 10.0);
        // The crease (x = 0) halves the front at the crotch line.
        let side_f = (front_hip + ext_f) / 2.0;
        // Shorts end on the thigh, with room to move.
        let hem_f = match legs {
            Legs::Shorts => {
                let thigh = (avatar.measure(Measure::Thigh).cm + SHORTS_HEM_EASE_CM) * 10.0;
                (thigh / 4.0 - 10.0).min(side_f)
            }
            _ => 0.1 * hips,
        };
        let hem_b = hem_f + 20.0;
        let out = side_f - hem_f;
        let cf = side_f - front_hip;
        let side_b = -(hem_b + out);
        let cb = side_b + back_hip;
        // Waist shaping: the centre front comes in 1 cm, the centre back
        // 2.5 cm and rises 1.5 cm; the side seams take the rest, the waist
        // fitting the body without ease (the trousers hang from it).
        // Jeans keep the back's top level for their straight waistband.
        let (cf_in, cb_in) = (10.0, 25.0);
        let cb_up = if legs == Legs::Jeans { 0.0 } else { 15.0 };
        let side_in = if legs.pulled_on() {
            0.0
        } else {
            ((front_hip + back_hip - waist / 2.0 - cf_in - cb_in) / 2.0).max(0.0)
        };
        let crotch_f = p(cf - ext_f, y_c);
        let inseam = crotch_f.distance(p(-hem_f, 0.0));
        let crotch_bx = cb + ext_b;
        let y_cb = (inseam * inseam - (crotch_bx - hem_b).powi(2))
            .max(0.0)
            .sqrt();
        let slant = cb_in / (len + cb_up - y_h);
        Ok(Block {
            len,
            rise,
            y_c,
            depth,
            y_h,
            hem_f,
            hem_b,
            ext_f,
            ext_b,
            out,
            side_in,
            cf,
            cf_in,
            cb,
            cb_in,
            cb_up,
            crotch_f,
            crotch_bx,
            y_cb,
            slant,
            waist_top,
        })
    }

    /// A side seam from the hem at `hem_x` up to the waist; `sign` +1 on
    /// the front (side on +x), −1 on the back.
    fn side(&self, hem_x: f64, sign: f64) -> CubicBez {
        let hem = p(hem_x, 0.0);
        let top = p(hem_x + sign * (self.out - self.side_in), self.len);
        let [c1, c2] = side_curve(hem, p(hem_x + sign * self.out, self.y_h), top, self.depth);
        CubicBez::new(hem, c1, c2, top)
    }

    /// The front's side seam, from the hem up.
    fn front_side(&self) -> CubicBez {
        self.side(self.hem_f, 1.0)
    }

    /// The back's side seam, from the hem up.
    fn back_side(&self) -> CubicBez {
        self.side(-self.hem_b, -1.0)
    }

    /// The front's top at the centre front.
    fn front_cf_top(&self) -> DVec2 {
        p(self.cf + self.cf_in, self.len)
    }

    /// The front's centre front, from the waist down to the crotch.
    fn front_cf(&self) -> CubicBez {
        CubicBez::new(
            self.front_cf_top(),
            p(self.cf, self.y_c + 0.3 * self.rise),
            p(self.cf - 0.35 * self.ext_f, self.y_c),
            self.crotch_f,
        )
    }

    /// The back's centre back, from the crotch up to its raised top.
    fn back_cb(&self) -> CubicBez {
        let y_cb = self.y_cb;
        CubicBez::new(
            p(self.crotch_bx, y_cb),
            p(self.cb + 0.35 * self.ext_b, y_cb),
            p(
                self.cb + self.slant * (self.y_h - y_cb - 0.35 * self.rise),
                y_cb + 0.35 * self.rise,
            ),
            p(self.cb - self.cb_in, self.len + self.cb_up),
        )
    }

    /// The height of the backs' raised top (cm).
    fn back_top(&self) -> f64 {
        self.waist_top + self.cb_up / 10.0
    }

    /// The height on the body of draft height `y` (cm).
    fn height(&self, y: f64) -> f64 {
        self.waist_top - (self.len - y) / 10.0
    }
}

/// A cubic's control points, as `Draft::panel` takes an edge's curve.
fn controls(c: &CubicBez) -> Option<[DVec2; 2]> {
    Some([c.p1, c.p2])
}

/// Trousers: for each leg a front and a back, the right leg's a mirror
/// copy of the left's. A straight-leg block without darts: the waist is
/// shaped at the side seams and the centre lines; the back is wider, with
/// a longer crotch extension and a centre back raised and slanted. The
/// side seams of front and back have the same shape, and the back's
/// crotch point is lowered until its inseam is as long as the front's,
/// so every seam matches. Joggers pull on: the waist is drafted as wide
/// as the hips and gathered to the body's waist by an elastic, the hems
/// to the ankles; the centre front is sewn all the way (no fly). Shorts
/// end on the thigh, wide enough for it.
fn trousers(d: &mut Draft, s: &Sizes, avatar: &Avatar, legs: Legs) -> Result<(), CommandError> {
    let b = Block::new(d, s, avatar, legs)?;
    if legs == Legs::Jeans {
        return jeans(d, &b);
    }
    let pulled_on = legs.pulled_on();
    let waist = mm("waist_cm", s.waist_cm)?;
    let front_side = b.front_side();
    let back_side = b.back_side().reversed();
    // Front, seen from outside on the left leg: the side seam on +x.
    // Edges: 0 hem, 1 side, 2 waist, 3 centre front (crotch), 4 inseam.
    let front = d.panel(
        "Front left",
        &[
            (p(-b.hem_f, 0.0), None),
            (p(b.hem_f, 0.0), controls(&front_side)),
            (front_side.p3, None),
            (b.front_cf_top(), controls(&b.front_cf())),
            (b.crotch_f, None),
        ],
    )?;
    // Back, seen from outside on the left leg: the side seam on −x.
    // Edges: 0 hem, 1 inseam, 2 centre back (crotch), 3 waist, 4 side.
    let cb = b.back_cb();
    let back = d.panel(
        "Back left",
        &[
            (p(-b.hem_b, 0.0), None),
            (p(b.hem_b, 0.0), None),
            (cb.p0, controls(&cb)),
            (cb.p3, None),
            (back_side.p0, controls(&back_side)),
        ],
    )?;
    let front_right = d.mirror(&front, "Front right")?;
    let back_right = d.mirror(&back, "Back right")?;
    for (f, bk) in [(&front, &back), (&front_right, &back_right)] {
        d.sew(f.edge(1)?, bk.edge(4)?)?;
        d.sew(f.edge(4)?, bk.edge(1)?)?;
    }
    if pulled_on {
        d.sew(front.edge(3)?, front_right.edge(3)?)?;
    } else {
        fly(d, &front, &front_right, 3)?;
    }
    d.sew(back.edge(2)?, back_right.edge(2)?)?;
    if pulled_on {
        let len = |d: &Draft, e: EdgeId| d.tx.project().edge_length(e).unwrap_or_default();
        // The waist gathered to the body's; the joggers' hems to the
        // ankles with room for the foot.
        let top = 2.0 * (len(d, front.edge(2)?) + len(d, back.edge(3)?));
        let ankle = (avatar.measure(Measure::Ankle).cm + 6.0) * 10.0;
        let hem = len(d, front.edge(0)?) + len(d, back.edge(0)?);
        for (f, bk) in [(&front, &back), (&front_right, &back_right)] {
            d.elastic(whole(f.edge(2)?), waist / top)?;
            d.elastic(whole(bk.edge(3)?), waist / top)?;
            if legs == Legs::Joggers {
                d.elastic(whole(f.edge(0)?), ankle / hem)?;
                d.elastic(whole(bk.edge(0)?), ankle / hem)?;
            }
        }
    }
    // Each piece's crease goes to the front or the back of its leg; the
    // backs' tops are their raised centres.
    for (piece, around, angle, top) in [
        (&front, Around::LeftLeg, 0.0, b.waist_top),
        (&back, Around::LeftLeg, 180.0, b.back_top()),
        (&front_right, Around::RightLeg, 0.0, b.waist_top),
        (&back_right, Around::RightLeg, 180.0, b.back_top()),
    ] {
        d.place_at(piece, around, angle, top, 30.0, Some(piece.origin_x))?;
    }
    Ok(())
}

/// The fly: a zipper from the waist down the centre front (edge `k` of
/// both fronts), the crotch seam closed below it. The left front's
/// centre front runs down from the waist, its mirror image up to it;
/// both sides of the zipper start at its bottom stop.
fn fly(d: &mut Draft, front: &Piece, front_right: &Piece, k: usize) -> Result<(), CommandError> {
    let cf_len =
        d.tx.project()
            .edge_length(front.edge(k)?)
            .unwrap_or_default();
    let fly = FLY_MM.min(0.6 * cf_len);
    d.zip(
        stretch(front.edge(k)?, None, Some(fly), true),
        stretch(front_right.edge(k)?, Some(cf_len - fly), None, false),
    )?;
    d.sew_sides(
        stretch(front.edge(k)?, Some(fly), None, false),
        stretch(front_right.edge(k)?, None, Some(cf_len - fly), true),
    )
}

/// Where the front pocket's mouth meets the side seam, below the waist,
/// and the waist, from the side (mm).
const POCKET_SIDE_MM: f64 = 90.0;
const POCKET_WAIST_MM: f64 = 110.0;
/// How far the pocket facing reaches below the mouth (mm).
const POCKET_BAG_MM: f64 = 70.0;
/// The back yoke's seam below the waist at the side and below the raised
/// top at the centre back (mm).
const JEANS_YOKE_SIDE_MM: f64 = 50.0;
const JEANS_YOKE_CB_MM: f64 = 80.0;
/// The waistband's height (mm).
const WAISTBAND_MM: f64 = 40.0;
/// A belt loop: its width, how far below the waist its lower end is
/// sewn, and its length, just over the band so that it lies flat (mm).
const LOOP_W_MM: f64 = 15.0;
const LOOP_BELOW_MM: f64 = 8.0;
const LOOP_MM: f64 = WAISTBAND_MM + LOOP_BELOW_MM + 4.0;
/// The back pockets: width, height at the sides, and the point's depth
/// below them (mm).
const BACK_POCKET_W_MM: f64 = 140.0;
const BACK_POCKET_H_MM: f64 = 150.0;
const BACK_POCKET_POINT_MM: f64 = 25.0;
/// The coin pocket (mm).
const COIN_W_MM: f64 = 60.0;
const COIN_H_MM: f64 = 55.0;

/// Jeans: the trousers' block in denim, closer at the hips, with five
/// pockets (two scooped at the front over their facings, a coin pocket on
/// the right facing, two patch pockets on the back), a yoke cut off the
/// back, a fly zipper, and a waistband with belt loops.
fn jeans(d: &mut Draft, b: &Block) -> Result<(), CommandError> {
    let len = b.len;
    // The front: its side seam cut where the pocket's mouth meets it.
    let side = b.front_side();
    let side_top = side.p3;
    let (side_low, side_up) = side.split(rising_to(&side, len - POCKET_SIDE_MM));
    let m1 = side_low.p3;
    let cf_top = b.front_cf_top();
    let m2 = p(
        side_top.x - POCKET_WAIST_MM.min(0.6 * (side_top.x - cf_top.x)),
        len,
    );
    let (w, hgt) = (m1.x - m2.x, len - m1.y);
    let mouth = [p(m1.x - 0.55 * w, m1.y), p(m2.x, m2.y - 0.55 * hgt)];
    // The facing's edge under the front, from the waist round below the
    // mouth to the side seam.
    let bag = [
        p(m2.x - 30.0, m1.y - 0.5 * POCKET_BAG_MM),
        p(m1.x - 0.5 * w, m1.y - POCKET_BAG_MM),
    ];
    // Front edges: 0 hem, 1 side, 2 pocket mouth, 3 waist, 4 centre
    // front, 5 inseam.
    let front = d.panel(
        "Front left",
        &[
            (p(-b.hem_f, 0.0), None),
            (p(b.hem_f, 0.0), controls(&side_low)),
            (m1, Some(mouth)),
            (m2, None),
            (cf_top, controls(&b.front_cf())),
            (b.crotch_f, None),
        ],
    )?;
    // Its lines, drawn before it is mirrored so that the right front has
    // them too: where the facing is sewn under it, and a belt loop's
    // lower end.
    let bag_line = d.line(&front, m2, &[(m1, Some(bag))])?;
    let front_loop_x = (m2.x - 25.0).max(cf_top.x + 40.0);
    let front_loop = d.line(
        &front,
        p(front_loop_x - LOOP_W_MM / 2.0, len - LOOP_BELOW_MM),
        &[(p(front_loop_x + LOOP_W_MM / 2.0, len - LOOP_BELOW_MM), None)],
    )?;
    // The facing: edges 0 side, 1 waist, 2 under the front.
    let facing = d.panel(
        "Pocket facing left",
        &[(m1, controls(&side_up)), (side_top, None), (m2, Some(bag))],
    )?;

    // The back, in two: a yoke above a slanted seam, and the back below.
    let side = b.back_side();
    let back_side_top = side.p3;
    let (bside_low, bside_up) = side.split(rising_to(&side, len - JEANS_YOKE_SIDE_MM));
    let ys = bside_low.p3;
    let cb = b.back_cb();
    let cb_top = cb.p3;
    let (cb_low, cb_up) = cb.split(rising_to(&cb, len + b.cb_up - JEANS_YOKE_CB_MM));
    let yc = cb_low.p3;
    // Back edges: 0 hem, 1 inseam, 2 centre back, 3 yoke seam, 4 side.
    let back = d.panel(
        "Back left",
        &[
            (p(-b.hem_b, 0.0), None),
            (p(b.hem_b, 0.0), None),
            (cb_low.p0, controls(&cb_low)),
            (yc, None),
            (ys, controls(&bside_low.reversed())),
        ],
    )?;
    // A patch pocket, its point down, between the side and the centre
    // back below the yoke.
    let pocket_w = BACK_POCKET_W_MM.min(0.5 * (yc.x - ys.x));
    let pocket_top = ys.y.min(yc.y) - 25.0;
    let pocket_x = (ys.x + yc.x) / 2.0 + 10.0;
    let (pl, pr) = (pocket_x - pocket_w / 2.0, pocket_x + pocket_w / 2.0);
    let pocket_low = pocket_top - BACK_POCKET_H_MM;
    let pocket = [
        p(pl, pocket_low + BACK_POCKET_POINT_MM),
        p(pocket_x, pocket_low),
        p(pr, pocket_low + BACK_POCKET_POINT_MM),
        p(pr, pocket_top),
        p(pl, pocket_top),
    ];
    let [q0, q1, q2, q3, q4] = pocket;
    let pocket_line = d.line(&back, q4, &[(q0, None), (q1, None), (q2, None), (q3, None)])?;
    // Yoke edges: 0 seam, 1 centre back, 2 waist, 3 side.
    let yoke = d.panel(
        "Yoke left",
        &[
            (ys, None),
            (yc, controls(&cb_up)),
            (cb_top, None),
            (back_side_top, controls(&bside_up.reversed())),
        ],
    )?;
    // Two belt loops on the yoke: by the side seam and by the centre back.
    let on_yoke = |u: f64| back_side_top.lerp(cb_top, u);
    let yoke_loops = [on_yoke(0.15), on_yoke(0.8)];
    let loop_line = |d: &mut Draft, at: DVec2| {
        let y = at.y - LOOP_BELOW_MM;
        d.line(
            &yoke,
            p(at.x - LOOP_W_MM / 2.0, y),
            &[(p(at.x + LOOP_W_MM / 2.0, y), None)],
        )
    };
    let yoke_lines = [loop_line(d, yoke_loops[0])?, loop_line(d, yoke_loops[1])?];
    // Pocket edges: 0, 1 the point, 2 right side, 3 top (open), 4 left.
    let patch = d.panel(
        "Back pocket left",
        &[(q0, None), (q1, None), (q2, None), (q3, None), (q4, None)],
    )?;
    // The loops, each drawn where it is sewn: edges 0 lower end, 1, 2
    // upper end, 3.
    let loop_at = |at: DVec2| {
        let (l, r, y) = (
            at.x - LOOP_W_MM / 2.0,
            at.x + LOOP_W_MM / 2.0,
            at.y - LOOP_BELOW_MM,
        );
        [
            (p(l, y), None),
            (p(r, y), None),
            (p(r, y + LOOP_MM), None),
            (p(l, y + LOOP_MM), None),
        ]
    };
    let loop_front = d.panel("Belt loop front left", &loop_at(p(front_loop_x, len)))?;
    let loop_side = d.panel("Belt loop side left", &loop_at(yoke_loops[0]))?;
    let loop_back = d.panel("Belt loop back left", &loop_at(yoke_loops[1]))?;

    // The waistband in two halves sewn at the centre back and the centre
    // front, their lower edges in one edge per waist they are sewn to.
    let edge_len = |d: &Draft, e: EdgeId| d.tx.project().edge_length(e).unwrap_or_default();
    let (lf, lfa, ly) = (
        edge_len(d, front.edge(3)?),
        edge_len(d, facing.edge(1)?),
        edge_len(d, yoke.edge(2)?),
    );
    let half = lf + lfa + ly;
    let around = 2.0 * half;
    let wb = WAISTBAND_MM;
    // Edges: 0 front, 1 facing, 2 yoke, 3 centre back, 4 top, 5 front end.
    let band_left = d.panel(
        "Waistband left",
        &[
            (p(0.0, 0.0), None),
            (p(lf, 0.0), None),
            (p(lf + lfa, 0.0), None),
            (p(half, 0.0), None),
            (p(half, wb), None),
            (p(0.0, wb), None),
        ],
    )?;
    // Edges: 0 yoke, 1 facing, 2 front, 3 front end, 4 top, 5 centre
    // back.
    let band_right = d.panel(
        "Waistband right",
        &[
            (p(0.0, 0.0), None),
            (p(ly, 0.0), None),
            (p(ly + lfa, 0.0), None),
            (p(half, 0.0), None),
            (p(half, wb), None),
            (p(0.0, wb), None),
        ],
    )?;

    let front_right = d.mirror(&front, "Front right")?;
    let facing_right = d.mirror(&facing, "Pocket facing right")?;
    let back_right = d.mirror(&back, "Back right")?;
    let yoke_right = d.mirror(&yoke, "Yoke right")?;
    let patch_right = d.mirror(&patch, "Back pocket right")?;
    let loop_front_right = d.mirror(&loop_front, "Belt loop front right")?;
    let loop_side_right = d.mirror(&loop_side, "Belt loop side right")?;
    let loop_back_right = d.mirror(&loop_back, "Belt loop back right")?;
    // The coin pocket on the right facing alone: that facing on its own.
    PanelUnlink {
        panel: facing_right.id,
    }
    .apply(d.tx)?;
    let coin_w = COIN_W_MM.min(0.55 * (side_top.x - m2.x));
    let (cr, ct) = (side_top.x - 15.0, len - 15.0);
    let (cl, cbm) = (cr - coin_w, ct - COIN_H_MM);
    let coin_line = d.line(
        &facing_right,
        p(cl, ct),
        &[(p(cl, cbm), None), (p(cr, cbm), None), (p(cr, ct), None)],
    )?;
    // Drawn reflected, as the right pieces are: edges 0 bottom, 1, 2 top
    // (open), 3.
    let coin = d.reflected(
        "Coin pocket",
        &[p(cr, cbm), p(cl, cbm), p(cl, ct), p(cr, ct)],
    )?;

    // Sewing, each side alike.
    let lines_of = |d: &Draft, piece: &Piece| -> Vec<r3d_core::InternalLineId> {
        d.tx.project()
            .internal_lines()
            .iter()
            .filter(|(_, l)| l.panel == piece.id)
            .map(|(id, _)| *id)
            .collect()
    };
    let front_lines = [bag_line, front_loop];
    let front_right_lines = lines_of(d, &front_right);
    let yoke_right_lines = lines_of(d, &yoke_right);
    let back_right_lines = lines_of(d, &back_right);
    let line = |lines: &[r3d_core::InternalLineId], k: usize| {
        lines
            .get(k)
            .copied()
            .ok_or_else(|| internal(format!("no line {k}")))
    };
    let sides = [
        (
            &front,
            &facing,
            &back,
            &yoke,
            &patch,
            [&loop_front, &loop_side, &loop_back],
            front_lines.to_vec(),
            yoke_lines.to_vec(),
            vec![pocket_line],
        ),
        (
            &front_right,
            &facing_right,
            &back_right,
            &yoke_right,
            &patch_right,
            [&loop_front_right, &loop_side_right, &loop_back_right],
            front_right_lines,
            yoke_right_lines,
            back_right_lines,
        ),
    ];
    for (f, fa, bk, y, pk, loops, f_lines, y_lines, b_lines) in &sides {
        // The side seam: the front then its facing, against the back
        // then its yoke.
        d.sew_chains(&[f.edge(1)?, fa.edge(0)?], &[bk.edge(4)?, y.edge(3)?])?;
        d.sew(f.edge(5)?, bk.edge(1)?)?;
        d.sew(y.edge(0)?, bk.edge(3)?)?;
        // The facing under the front, the pocket on the back, the loops'
        // lower ends.
        d.overlay(fa, whole(fa.edge(2)?), f, line_side(line(f_lines, 0)?))?;
        let pocket_run = d.run(pk, &[4, 0, 1, 2])?;
        d.overlay(pk, pocket_run, bk, line_side(line(b_lines, 0)?))?;
        d.overlay(
            loops[0],
            whole(loops[0].edge(0)?),
            f,
            line_side(line(f_lines, 1)?),
        )?;
        d.overlay(
            loops[1],
            whole(loops[1].edge(0)?),
            y,
            line_side(line(y_lines, 0)?),
        )?;
        d.overlay(
            loops[2],
            whole(loops[2].edge(0)?),
            y,
            line_side(line(y_lines, 1)?),
        )?;
        d.topstitch(f.edge(2)?, 2, 1.5)?;
        d.topstitch(y.edge(0)?, 2, 1.5)?;
        d.topstitch(pk.edge(3)?, 2, 1.5)?;
    }
    let coin_run = d.run(&coin, &[3, 0, 1])?;
    d.overlay(&coin, coin_run, &facing_right, line_side(coin_line))?;
    d.topstitch(coin.edge(2)?, 1, 1.5)?;
    fly(d, &front, &front_right, 4)?;
    d.sew(back.edge(2)?, back_right.edge(2)?)?;
    d.sew(yoke.edge(1)?, yoke_right.edge(1)?)?;
    // The waistband over each waist, round from the left centre front.
    for (band, waist) in [
        (band_left.edge(0)?, front.edge(3)?),
        (band_left.edge(1)?, facing.edge(1)?),
        (band_left.edge(2)?, yoke.edge(2)?),
        (band_right.edge(0)?, yoke_right.edge(2)?),
        (band_right.edge(1)?, facing_right.edge(1)?),
        (band_right.edge(2)?, front_right.edge(3)?),
        (band_left.edge(3)?, band_right.edge(5)?),
        (band_left.edge(5)?, band_right.edge(3)?),
    ] {
        d.sew(band, waist)?;
    }
    // The loops' upper ends over the waistband's top: a point's place
    // round the waist, from the left centre front, on each piece, and on
    // the band's top (walked from the right).
    let round_front = |x: f64| x - cf_top.x;
    let yoke_len = (cb_top - back_side_top).length();
    let round_yoke =
        |x: f64| lf + lfa + yoke_len * (x - back_side_top.x) / (cb_top.x - back_side_top.x);
    let on_band = |s: f64| -> Result<(EdgeId, f64), CommandError> {
        Ok(if s <= half {
            (band_left.edge(4)?, half - s)
        } else {
            (band_right.edge(4)?, around - s)
        })
    };
    for (loops, mirror) in [
        ([&loop_front, &loop_side, &loop_back], false),
        (
            [&loop_front_right, &loop_side_right, &loop_back_right],
            true,
        ),
    ] {
        for (k, lp) in loops.into_iter().enumerate() {
            let Some((a, z)) = d.ends(lp, lp.edge(2)?) else {
                continue;
            };
            let round = |x: f64| {
                let s = if k == 0 {
                    round_front(x)
                } else {
                    round_yoke(x)
                };
                if mirror { around - s } else { s }
            };
            let ((top, s0), (_, s1)) = (on_band(round(a.x))?, on_band(round(z.x))?);
            let side = if s0 <= s1 {
                stretch(top, Some(s0), Some(s1), false)
            } else {
                stretch(top, Some(s1), Some(s0), true)
            };
            d.sew_sides(whole(lp.edge(2)?), side)?;
        }
    }

    // Each piece where it is worn: the fronts' and backs' creases at the
    // front and back of their legs, every other piece as drawn on them.
    let back_low = b.height(ys.y.max(yc.y));
    let pocket_top_h = b.height(pocket_top);
    let loop_top = |at: DVec2| b.height(at.y - LOOP_BELOW_MM + LOOP_MM);
    for (pieces, around_leg) in [
        (
            [
                &front,
                &facing,
                &back,
                &yoke,
                &patch,
                &loop_front,
                &loop_side,
                &loop_back,
            ],
            Around::LeftLeg,
        ),
        (
            [
                &front_right,
                &facing_right,
                &back_right,
                &yoke_right,
                &patch_right,
                &loop_front_right,
                &loop_side_right,
                &loop_back_right,
            ],
            Around::RightLeg,
        ),
    ] {
        let [f, fa, bk, y, pk, lf, ls, lb] = pieces;
        for (piece, angle, top, offset) in [
            (f, 0.0, b.waist_top, 30.0),
            (fa, 0.0, b.waist_top, 22.0),
            (bk, 180.0, back_low, 30.0),
            (y, 180.0, b.back_top(), 30.0),
            (pk, 180.0, pocket_top_h, 40.0),
            (lf, 0.0, loop_top(p(front_loop_x, len)), 42.0),
            (ls, 180.0, loop_top(yoke_loops[0]), 42.0),
            (lb, 180.0, loop_top(yoke_loops[1]), 42.0),
        ] {
            d.place_at(piece, around_leg, angle, top, offset, Some(piece.origin_x))?;
        }
    }
    d.place_at(
        &coin,
        Around::RightLeg,
        0.0,
        b.height(len - 15.0),
        26.0,
        Some(coin.origin_x),
    )?;
    // The waistband round the body from the centre back.
    let band_top = b.waist_top + (b.cb_up / 2.0 + wb) / 10.0;
    d.place_at(
        &band_left,
        Around::Body,
        180.0,
        band_top,
        38.0,
        Some(band_left.origin_x + half),
    )?;
    d.place_at(
        &band_right,
        Around::Body,
        180.0,
        band_top,
        38.0,
        Some(band_right.origin_x),
    )?;
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
    let neck_height = Stature::of(body).neck * h;
    // Quarter widths: chest, waist (twice the ease: it hangs from the
    // shoulders, gently shaped, not fitted), hips, hem.
    let w = (chest + ease) / 4.0;
    let a = (waist + 2.0 * ease) / 4.0;
    let b = (hips + ease) / 4.0;
    let upper = Upper::of(d.tx.project())?;
    let shoulder = upper.shoulder_half(chest);
    let neck = upper.neck_half();
    let y_s = len - upper.shoulder_drop();
    let y_a = y_s - upper.armhole_depth();
    // Neck to waist on the body, then waist to hips.
    let y_w = (len - (neck_height - waist_height_cm(d.tx.project())?) * 10.0).max(0.4 * len);
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
    d.place(&front, Around::Body, 0.0, neck_height + TOP_LIFT_CM)?;
    d.place(&back, Around::Body, 180.0, neck_height + TOP_LIFT_CM)?;
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
    d.place_at(&front, Around::Flat, 0.0, TOTE_TOP_CM, TOTE_OUT_MM, None)?;
    d.place_at(
        &back,
        Around::Flat,
        0.0,
        TOTE_TOP_CM,
        TOTE_OUT_MM - TOTE_LAYERS_MM,
        None,
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
