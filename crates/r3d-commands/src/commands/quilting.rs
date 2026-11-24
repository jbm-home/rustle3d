//! Quilting and padding (`docs/specs/31-construction-details-2.md`).

use r3d_core::detail::{LOFT_MM, PADDING_GSM, QUILT_SPACING_MM, STITCH_MM};
use r3d_core::{
    Detail, DetailId, InternalLineId, PanelId, Project, QuiltPattern, Quilting, Transaction,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::get_panel;
use super::interfacing::overlap;
use super::topstitch::DEFAULT_STITCH_MM;
use crate::error::{CommandError, Invalid, finite};
use crate::spec::{CommandOutput, CommandSpec, Described};

/// A quilting's values when none are given: 5 cm diamonds over an 8 mm,
/// 100 g/m² padding.
pub const DEFAULT_QUILT_SPACING_MM: f64 = 50.0;
pub const DEFAULT_LOFT_MM: f64 = 8.0;
pub const DEFAULT_PADDING_GSM: f64 = 100.0;

pub(crate) fn get_quilting(p: &Project, id: DetailId) -> Result<Quilting, CommandError> {
    match p.details().get(&id) {
        Some(Detail::Quilting(q)) => Ok(q.clone()),
        Some(_) => Err(CommandError::invalid("quilting", Invalid::OtherKind)),
        None => Err(CommandError::not_found(id)),
    }
}

fn within(field: &'static str, v: f64, (lo, hi): (f64, f64)) -> Result<(), CommandError> {
    finite(field, &[v])?;
    if (lo..=hi).contains(&v) {
        Ok(())
    } else {
        Err(CommandError::invalid(field, Invalid::Range(lo, hi)))
    }
}

/// `q` checked: its values, its area, no other quilting on the piece
/// overlapping it (one per area).
fn checked(p: &Project, q: Quilting, except: Option<DetailId>) -> Result<Quilting, CommandError> {
    get_panel(p, q.panel)?;
    within("spacing_mm", q.spacing_mm, QUILT_SPACING_MM)?;
    within("loft_mm", q.loft_mm, LOFT_MM)?;
    within("padding_gsm", q.padding_gsm, PADDING_GSM)?;
    within("stitch_mm", q.stitch_mm, STITCH_MM)?;
    finite("angle_deg", &[q.angle_deg])?;
    if !q.valid() {
        return Err(CommandError::invalid("color", Invalid::Format));
    }
    if let Some(line) = q.line {
        p.internal_line(line)
            .ok_or_else(|| CommandError::not_found(line))?;
    }
    if p.quilting_outline(&q).is_none() {
        return Err(CommandError::invalid("line", Invalid::NotClosed));
    }
    let outline = p.quilting_outline(&q).unwrap_or_default();
    let clash = p
        .quiltings()
        .filter(|(id, o)| Some(*id) != except && o.panel == q.panel)
        .filter_map(|(_, o)| p.quilting_outline(o))
        .any(|o| overlap(&outline, &o));
    if clash {
        return Err(CommandError::invalid("panel", Invalid::Overlaps));
    }
    Ok(q)
}

/// Quilt a piece, or the inside of a closed line on it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct QuiltingAdd {
    pub panel: PanelId,
    /// Only inside this closed internal line. Omitted: the whole piece.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<InternalLineId>,
    /// `channels` (parallel lines) or `diamonds`. Omitted: diamonds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pattern: Option<QuiltPattern>,
    /// The lines' angle from the grainline (degrees; each side of it for
    /// diamonds). Omitted: 45 for diamonds, 0 for channels.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub angle_deg: Option<f64>,
    /// The distance between parallel lines (15–150 mm). Omitted: 50.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spacing_mm: Option<f64>,
    /// The padding's thickness (1–30 mm). Omitted: 8.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub loft_mm: Option<f64>,
    /// The padding's weight (10–600 g/m²). Omitted: 100.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub padding_gsm: Option<f64>,
    /// The stitches' length (1–8 mm). Omitted: 3.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stitch_mm: Option<f64>,
    /// The thread's colour, `#RRGGBB`. Omitted: a darker shade of the
    /// garment's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
}

impl Described for QuiltingAdd {
    const SUMMARY: &'static str = "Quilt a piece (or the inside of a closed internal `line` on it) over a padding: `pattern` channels or diamonds (default), `angle_deg` from the grainline (default 45 for diamonds, 0 for channels), lines `spacing_mm` apart (default 50), a padding `loft_mm` thick (default 8) weighing `padding_gsm` (default 100), `stitch_mm` stitches and a thread `color`. The cloth there drapes as shell, padding and lining together, folds along the stitched lines, and puffs out between them in the views. One quilting per area. Returns the quilting (a detail).";
}

impl CommandSpec for QuiltingAdd {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        let pattern = self.pattern.unwrap_or_default();
        let angle = match pattern {
            QuiltPattern::Diamonds => 45.0,
            QuiltPattern::Channels => 0.0,
        };
        let q = checked(
            tx.project(),
            Quilting {
                panel: self.panel,
                line: self.line,
                pattern,
                angle_deg: self.angle_deg.unwrap_or(angle),
                spacing_mm: self.spacing_mm.unwrap_or(DEFAULT_QUILT_SPACING_MM),
                loft_mm: self.loft_mm.unwrap_or(DEFAULT_LOFT_MM),
                padding_gsm: self.padding_gsm.unwrap_or(DEFAULT_PADDING_GSM),
                stitch_mm: self.stitch_mm.unwrap_or(DEFAULT_STITCH_MM),
                color: self.color.clone(),
            },
            None,
        )?;
        let id = tx.new_detail_id();
        tx.put_detail(id, Detail::Quilting(q));
        Ok(CommandOutput {
            created: vec![id.into()],
        })
    }
}

/// Change a quilting.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct QuiltingSet {
    pub quilting: DetailId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pattern: Option<QuiltPattern>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub angle_deg: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spacing_mm: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub loft_mm: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub padding_gsm: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stitch_mm: Option<f64>,
    /// `#RRGGBB`, or `""` for the garment's shade.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
}

impl Described for QuiltingSet {
    const SUMMARY: &'static str = "Change a quilting: its `pattern`, `angle_deg`, `spacing_mm`, `loft_mm`, `padding_gsm`, `stitch_mm` or `color` (\"\" for a darker shade of the garment's); what is omitted stays.";
}

impl CommandSpec for QuiltingSet {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        let p = tx.project();
        let q = get_quilting(p, self.quilting)?;
        let q = checked(
            p,
            Quilting {
                pattern: self.pattern.unwrap_or(q.pattern),
                angle_deg: self.angle_deg.unwrap_or(q.angle_deg),
                spacing_mm: self.spacing_mm.unwrap_or(q.spacing_mm),
                loft_mm: self.loft_mm.unwrap_or(q.loft_mm),
                padding_gsm: self.padding_gsm.unwrap_or(q.padding_gsm),
                stitch_mm: self.stitch_mm.unwrap_or(q.stitch_mm),
                color: match self.color.as_deref() {
                    Some("") => None,
                    Some(c) => Some(c.to_owned()),
                    None => q.color.clone(),
                },
                ..q
            },
            Some(self.quilting),
        )?;
        tx.put_detail(self.quilting, Detail::Quilting(q));
        Ok(CommandOutput::default())
    }

    fn merge_key(&self) -> Option<String> {
        Some(format!("quilting.set:{}", self.quilting))
    }
}

/// Remove a quilting.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct QuiltingRemove {
    pub quilting: DetailId,
}

impl Described for QuiltingRemove {
    const SUMMARY: &'static str = "Remove a quilting and its padding.";
}

impl CommandSpec for QuiltingRemove {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        get_quilting(tx.project(), self.quilting)?;
        tx.remove_detail(self.quilting);
        Ok(CommandOutput::default())
    }
}
