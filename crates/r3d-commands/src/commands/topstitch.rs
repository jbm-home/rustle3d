//! Topstitching (`docs/specs/31-construction-details-2.md`).

use r3d_core::detail::{STITCH_MM, TOPSTITCH_GAP_MM, TOPSTITCH_OFFSET_MM, TOPSTITCH_ROWS};
use r3d_core::{Detail, DetailId, Project, Thread, Topstitch, Transaction};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::seam::SeamSideSpec;
use crate::error::{CommandError, Invalid, finite};
use crate::spec::{CommandOutput, CommandSpec, Described};

/// A topstitch's values when none are given: one row 6 mm from its side,
/// rows 6 mm apart, 3 mm stitches.
pub const DEFAULT_TOPSTITCH_OFFSET_MM: f64 = 6.0;
pub const DEFAULT_TOPSTITCH_GAP_MM: f64 = 6.0;
pub const DEFAULT_STITCH_MM: f64 = 3.0;

pub(crate) fn get_topstitch(p: &Project, id: DetailId) -> Result<Topstitch, CommandError> {
    match p.details().get(&id) {
        Some(Detail::Topstitch(t)) => Ok(t.clone()),
        Some(_) => Err(CommandError::invalid("topstitch", Invalid::OtherKind)),
        None => Err(CommandError::not_found(id)),
    }
}

/// `t` checked field by field, and with rows on its piece.
fn checked(p: &Project, t: Topstitch) -> Result<Topstitch, CommandError> {
    finite("offset_mm", &[t.offset_mm])?;
    finite("gap_mm", &[t.gap_mm])?;
    finite("stitch_mm", &[t.stitch_mm])?;
    if !(1..=TOPSTITCH_ROWS).contains(&t.rows) {
        return Err(CommandError::invalid(
            "rows",
            Invalid::Range(1.0, f64::from(TOPSTITCH_ROWS)),
        ));
    }
    let least = if t.side.line.is_some() {
        -TOPSTITCH_OFFSET_MM
    } else {
        1.0
    };
    if !(least..=TOPSTITCH_OFFSET_MM).contains(&t.offset_mm) {
        return Err(CommandError::invalid(
            "offset_mm",
            Invalid::Range(least, TOPSTITCH_OFFSET_MM),
        ));
    }
    if !(TOPSTITCH_GAP_MM.0..=TOPSTITCH_GAP_MM.1).contains(&t.gap_mm) {
        return Err(CommandError::invalid(
            "gap_mm",
            Invalid::Range(TOPSTITCH_GAP_MM.0, TOPSTITCH_GAP_MM.1),
        ));
    }
    if !(STITCH_MM.0..=STITCH_MM.1).contains(&t.stitch_mm) {
        return Err(CommandError::invalid(
            "stitch_mm",
            Invalid::Range(STITCH_MM.0, STITCH_MM.1),
        ));
    }
    if !t.valid() {
        return Err(CommandError::invalid("color", Invalid::Format));
    }
    if p.topstitch_rows(&t).is_empty() {
        return Err(CommandError::invalid("offset_mm", Invalid::OutsidePiece));
    }
    Ok(t)
}

/// Add topstitching along a side.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TopstitchAdd {
    /// Where it runs, as a seam's side (the edge of the piece the
    /// stitches show on).
    pub side: SeamSideSpec,
    /// How many rows (1–3). Omitted: 1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rows: Option<u8>,
    /// The first row's distance from the side (mm). On outline edges
    /// 1–50, inside the piece; on an internal line −50–50, on its left
    /// (from its start) when positive. Omitted: 6.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offset_mm: Option<f64>,
    /// The distance between rows (1–30 mm). Omitted: 6.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gap_mm: Option<f64>,
    /// The stitch length (1–8 mm). Omitted: 3.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stitch_mm: Option<f64>,
    /// The thread's colour, `#RRGGBB`. Omitted: a darker shade of the
    /// garment's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    /// `normal` or `heavy` (topstitching thread). Omitted: normal.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thread: Option<Thread>,
}

impl Described for TopstitchAdd {
    const SUMMARY: &'static str = "Topstitch along a side (given as a seam's: a whole `edge`, consecutive outline `edges` or an internal `line`, maybe only `from_mm`/`to_mm` along it): `rows` (1–3, default 1), the first `offset_mm` from the side (default 6; inside the piece on outline edges, on the line's left when positive), the next `gap_mm` further (default 6), `stitch_mm` stitches (default 3), a `color` (#RRGGBB, default a darker shade of the garment's) and a `thread` (normal or heavy). Beside a seam, topstitch the edge of the piece the stitches show on. Rows are cut where they leave the piece. The cloth under them bends a little harder. Returns the topstitch (a detail).";
}

impl CommandSpec for TopstitchAdd {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        let p = tx.project();
        let side = self.side.side("side", p)?;
        let t = checked(
            p,
            Topstitch {
                side,
                rows: self.rows.unwrap_or(1),
                offset_mm: self.offset_mm.unwrap_or(DEFAULT_TOPSTITCH_OFFSET_MM),
                gap_mm: self.gap_mm.unwrap_or(DEFAULT_TOPSTITCH_GAP_MM),
                stitch_mm: self.stitch_mm.unwrap_or(DEFAULT_STITCH_MM),
                color: self.color.clone(),
                thread: self.thread.unwrap_or_default(),
            },
        )?;
        let id = tx.new_detail_id();
        tx.put_detail(id, Detail::Topstitch(t));
        Ok(CommandOutput {
            created: vec![id.into()],
        })
    }
}

/// Change a topstitch.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TopstitchSet {
    pub topstitch: DetailId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rows: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offset_mm: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gap_mm: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stitch_mm: Option<f64>,
    /// `#RRGGBB`, or `""` for the garment's shade.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thread: Option<Thread>,
}

impl Described for TopstitchSet {
    const SUMMARY: &'static str = "Change a topstitch: its `rows`, `offset_mm`, `gap_mm`, `stitch_mm`, `color` (\"\" for a darker shade of the garment's) or `thread`; what is omitted stays.";
}

impl CommandSpec for TopstitchSet {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        let p = tx.project();
        let t = get_topstitch(p, self.topstitch)?;
        let t = checked(
            p,
            Topstitch {
                rows: self.rows.unwrap_or(t.rows),
                offset_mm: self.offset_mm.unwrap_or(t.offset_mm),
                gap_mm: self.gap_mm.unwrap_or(t.gap_mm),
                stitch_mm: self.stitch_mm.unwrap_or(t.stitch_mm),
                color: match self.color.as_deref() {
                    Some("") => None,
                    Some(c) => Some(c.to_owned()),
                    None => t.color.clone(),
                },
                thread: self.thread.unwrap_or(t.thread),
                side: t.side,
            },
        )?;
        tx.put_detail(self.topstitch, Detail::Topstitch(t));
        Ok(CommandOutput::default())
    }

    fn merge_key(&self) -> Option<String> {
        Some(format!("topstitch.set:{}", self.topstitch))
    }
}

/// Remove a topstitch.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TopstitchRemove {
    pub topstitch: DetailId,
}

impl Described for TopstitchRemove {
    const SUMMARY: &'static str = "Remove a topstitch.";
}

impl CommandSpec for TopstitchRemove {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        get_topstitch(tx.project(), self.topstitch)?;
        tx.remove_detail(self.topstitch);
        Ok(CommandOutput::default())
    }
}
