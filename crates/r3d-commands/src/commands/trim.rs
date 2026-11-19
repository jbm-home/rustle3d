//! Buttons and zippers (`docs/specs/26-trims.md`).

use r3d_core::{Button, ButtonId, Buttonhole, PanelId, Project, Transaction, Zipper, ZipperId};
use r3d_geom::{DVec2, polygon};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::seam::{SeamSideSpec, default_reversed, free_to_join};
use super::{Vec2, get_panel, v};
use crate::error::{CommandError, Invalid, finite};
use crate::spec::{CommandOutput, CommandSpec, Described};

/// A button's diameter when none is given (mm): a common shirt button.
pub const DEFAULT_BUTTON_MM: f64 = 11.0;
/// The largest button accepted (mm).
pub const MAX_BUTTON_MM: f64 = 80.0;

/// `at` must be inside `panel`'s outline (and outside its holes).
fn inside(p: &Project, field: &'static str, panel: PanelId, at: DVec2) -> Result<(), CommandError> {
    let piece = get_panel(p, panel)?;
    let outline = p
        .panel_outline(panel)
        .ok_or_else(|| CommandError::not_found(panel))?;
    let in_hole = piece.holes.iter().any(|hole| {
        let pts: Vec<DVec2> = hole
            .iter()
            .filter_map(|e| p.edge(*e).and_then(|e| p.point(e.start)).map(|pt| pt.pos))
            .collect();
        polygon::contains(&pts, at)
    });
    if polygon::contains(&outline, at) && !in_hole {
        Ok(())
    } else {
        Err(CommandError::invalid(field, Invalid::OutsidePiece))
    }
}

fn diameter(mm: f64) -> Result<f64, CommandError> {
    finite("diameter_mm", &[mm])?;
    if mm > 0.0 && mm <= MAX_BUTTON_MM {
        Ok(mm)
    } else {
        Err(CommandError::invalid(
            "diameter_mm",
            Invalid::Range(0.0, MAX_BUTTON_MM),
        ))
    }
}

/// The slit runs across the piece towards its middle: from the edge the
/// button sits near, inward.
fn default_angle(p: &Project, panel: PanelId, at: DVec2) -> f64 {
    let centre = p
        .panel_bounding_box(panel)
        .map_or(at, |b| (b.min + b.max) / 2.0);
    if centre.x >= at.x { 0.0 } else { 180.0 }
}

fn get_button(p: &Project, id: ButtonId) -> Result<Button, CommandError> {
    p.buttons()
        .get(&id)
        .cloned()
        .ok_or_else(|| CommandError::not_found(id))
}

fn get_zipper(p: &Project, id: ZipperId) -> Result<Zipper, CommandError> {
    p.zippers()
        .get(&id)
        .cloned()
        .ok_or_else(|| CommandError::not_found(id))
}

/// Check a button's points and size.
fn checked(p: &Project, b: &Button) -> Result<(), CommandError> {
    finite("at", &[b.at.x, b.at.y])?;
    finite("hole_at", &[b.hole.at.x, b.hole.at.y])?;
    finite("hole_angle_deg", &[b.hole.angle_deg])?;
    diameter(b.diameter_mm)?;
    inside(p, "at", b.panel, b.at)?;
    inside(p, "hole_at", b.hole.panel, b.hole.at)?;
    if b.panel == b.hole.panel && b.at.distance(b.hole.at) < 1e-6 {
        return Err(CommandError::invalid("hole_at", Invalid::SamePoint));
    }
    Ok(())
}

/// Add a button and its buttonhole.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ButtonAdd {
    /// The piece the button is sewn on.
    pub panel: PanelId,
    /// Where it is sewn, `[x, y]` in mm (inside the piece).
    pub at: Vec2,
    /// Its diameter (mm). Omitted: 11 mm.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diameter_mm: Option<f64>,
    /// The piece the buttonhole is cut in (usually the overlapping one).
    pub hole_panel: PanelId,
    /// Where the button sits in the buttonhole once buttoned: the slit's
    /// end nearest the edge, `[x, y]` in mm (inside the piece).
    pub hole_at: Vec2,
    /// Direction of the slit from `hole_at` (degrees from +X). Omitted:
    /// across the piece towards its middle.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hole_angle_deg: Option<f64>,
    /// Buttoned when draped. Omitted: buttoned.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub closed: Option<bool>,
}

impl Described for ButtonAdd {
    const SUMMARY: &'static str = "Add a button sewn on a piece at `at` and its buttonhole on `hole_panel` (often the overlapping piece; the same piece for a cuff). `hole_at` is where the button sits once buttoned (the slit's end nearest the edge); the slit runs from there along `hole_angle_deg` for the button's diameter + 3 mm. Buttoned, the drape holds the button point and `hole_at` together. Both points must be inside their pieces. Returns the button.";
}

impl CommandSpec for ButtonAdd {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        let p = tx.project();
        finite("hole_at", &self.hole_at)?;
        let hole_at = v(self.hole_at);
        get_panel(p, self.hole_panel)?;
        let button = Button {
            panel: self.panel,
            at: v(self.at),
            diameter_mm: self.diameter_mm.unwrap_or(DEFAULT_BUTTON_MM),
            hole: Buttonhole {
                panel: self.hole_panel,
                at: hole_at,
                angle_deg: self
                    .hole_angle_deg
                    .unwrap_or_else(|| default_angle(p, self.hole_panel, hole_at)),
            },
            closed: self.closed.unwrap_or(true),
        };
        checked(p, &button)?;
        let id = tx.new_button_id();
        tx.put_button(id, button);
        Ok(CommandOutput {
            created: vec![id.into()],
        })
    }
}

/// Move or resize a button or its buttonhole.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ButtonSet {
    pub button: ButtonId,
    /// Where the button is sewn, `[x, y]` in mm.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at: Option<Vec2>,
    /// Its diameter (mm).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diameter_mm: Option<f64>,
    /// Where it sits in the buttonhole, `[x, y]` in mm.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hole_at: Option<Vec2>,
    /// The slit's direction (degrees from +X).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hole_angle_deg: Option<f64>,
}

impl Described for ButtonSet {
    const SUMMARY: &'static str = "Move a button (`at`), its buttonhole (`hole_at`, `hole_angle_deg`) or change its diameter; what is omitted stays.";
}

impl CommandSpec for ButtonSet {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        let mut b = get_button(tx.project(), self.button)?;
        if let Some(at) = self.at {
            finite("at", &at)?;
            b.at = v(at);
        }
        if let Some(at) = self.hole_at {
            finite("hole_at", &at)?;
            b.hole.at = v(at);
        }
        if let Some(d) = self.diameter_mm {
            b.diameter_mm = d;
        }
        if let Some(a) = self.hole_angle_deg {
            b.hole.angle_deg = a;
        }
        checked(tx.project(), &b)?;
        tx.put_button(self.button, b);
        Ok(CommandOutput::default())
    }

    fn merge_key(&self) -> Option<String> {
        Some(format!("button.set:{}", self.button))
    }
}

/// Button or unbutton.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ButtonSetClosed {
    /// One or more buttons.
    pub buttons: Vec<ButtonId>,
    pub closed: bool,
}

impl Described for ButtonSetClosed {
    const SUMMARY: &'static str = "Button (`closed: true`) or unbutton buttons: buttoned, the drape holds each button and its buttonhole together; unbuttoned, they hang free. A running drape follows without starting again.";
}

impl CommandSpec for ButtonSetClosed {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        if self.buttons.is_empty() {
            return Err(CommandError::invalid("buttons", Invalid::Empty));
        }
        for id in &self.buttons {
            let b = get_button(tx.project(), *id)?;
            if b.closed != self.closed {
                tx.put_button(
                    *id,
                    Button {
                        closed: self.closed,
                        ..b
                    },
                );
            }
        }
        Ok(CommandOutput::default())
    }
}

/// Remove a button and its buttonhole.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ButtonRemove {
    pub button: ButtonId,
}

impl Described for ButtonRemove {
    const SUMMARY: &'static str = "Remove a button and its buttonhole.";
}

impl CommandSpec for ButtonRemove {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        tx.remove_button(self.button)
            .ok_or_else(|| CommandError::not_found(self.button))?;
        Ok(CommandOutput::default())
    }
}

/// Add a zipper between two sides.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ZipperAdd {
    /// Its sides, as a seam's; each starts at the bottom stop.
    pub a: SeamSideSpec,
    pub b: SeamSideSpec,
    /// How much of it is open from the top (mm). Omitted: closed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub open_mm: Option<f64>,
}

impl Described for ZipperAdd {
    const SUMMARY: &'static str = "Add a zipper joining two sides (given as a seam's: a whole `edge`, consecutive `edges` or an internal `line`, maybe only `from_mm`/`to_mm` along it). Both sides start at the zipper's bottom stop: use `reversed` to walk a side from its end (b.reversed omitted: chosen as for a seam). The drape joins the sides like a seam from the bottom stop up to the slider, `open_mm` below the top (omitted: 0, closed all along). Stretches sewn or zipped already cannot be used. Returns the zipper.";
}

impl CommandSpec for ZipperAdd {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        let p = tx.project();
        let mut a = self.a.side("a", p)?;
        let mut b = self.b.side("b", p)?;
        free_to_join(
            p,
            &p.side_spans(&a).unwrap_or_default(),
            &p.side_spans(&b).unwrap_or_default(),
        )?;
        a.reversed = self.a.reversed.unwrap_or(false);
        b.reversed = self
            .b
            .reversed
            .unwrap_or_else(|| default_reversed(p, &a, &b));
        let open_mm = open(self.open_mm.unwrap_or(0.0))?;
        let id = tx.new_zipper_id();
        tx.put_zipper(id, Zipper { a, b, open_mm });
        Ok(CommandOutput {
            created: vec![id.into()],
        })
    }
}

/// An open length, checked.
fn open(mm: f64) -> Result<f64, CommandError> {
    finite("open_mm", &[mm])?;
    if mm < 0.0 {
        return Err(CommandError::invalid("open_mm", Invalid::Negative));
    }
    Ok(mm)
}

/// Move a zipper's slider.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ZipperSetOpen {
    pub zipper: ZipperId,
    /// How much of it is open from the top (mm): 0 closed; its length (or
    /// more) open all the way.
    pub open_mm: f64,
}

impl Described for ZipperSetOpen {
    const SUMMARY: &'static str = "Move a zipper's slider: `open_mm` of it open from the top (0: closed; its length or more: open all the way). A running drape follows without starting again.";
}

impl CommandSpec for ZipperSetOpen {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        let z = get_zipper(tx.project(), self.zipper)?;
        let open_mm = open(self.open_mm)?;
        tx.put_zipper(self.zipper, Zipper { open_mm, ..z });
        Ok(CommandOutput::default())
    }

    fn merge_key(&self) -> Option<String> {
        Some(format!("zipper.set_open:{}", self.zipper))
    }
}

/// Remove a zipper.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ZipperRemove {
    pub zipper: ZipperId,
}

impl Described for ZipperRemove {
    const SUMMARY: &'static str = "Remove a zipper: its sides are free again.";
}

impl CommandSpec for ZipperRemove {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        tx.remove_zipper(self.zipper)
            .ok_or_else(|| CommandError::not_found(self.zipper))?;
        Ok(CommandOutput::default())
    }
}

/// After a step: drop the buttons whose pieces are gone and the zippers
/// whose sides are. The changes join the step's undo step.
pub(crate) fn settle_trims(tx: &mut Transaction<'_>) {
    let p = tx.project();
    let buttons: Vec<ButtonId> = p
        .buttons()
        .iter()
        .filter(|(_, b)| p.panel(b.panel).is_none() || p.panel(b.hole.panel).is_none())
        .map(|(id, _)| *id)
        .collect();
    let zippers: Vec<ZipperId> = p
        .zippers()
        .iter()
        .filter(|(_, z)| !p.zipper_length(z).is_some_and(|len| len > 0.0))
        .map(|(id, _)| *id)
        .collect();
    for id in buttons {
        tx.remove_button(id);
    }
    for id in zippers {
        tx.remove_zipper(id);
    }
}
