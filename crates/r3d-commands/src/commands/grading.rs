//! Sizes and grading (`docs/specs/18-grading.md`): a size range on the
//! project, a grade rule on each point, the size draped in 3D.

use std::collections::BTreeMap;

use r3d_core::{
    Avatar, Body, Edge, EdgeId, GarmentId, Grading, Point, PointId, Project, Size, Transaction,
};
use r3d_geom::DVec2;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::avatar::validate_body;
use super::{Vec2, v};
use crate::error::{CommandError, Invalid, finite, non_blank};
use crate::progress::Progress;
use crate::spec::{CommandOutput, CommandSpec, Described};

/// One size of a range, as commands take it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SizeSpec {
    /// As on the label: "38", "M".
    pub name: String,
    /// How the size's body differs from the base size's (cm; default 0).
    #[serde(default)]
    pub height_cm: f64,
    #[serde(default)]
    pub bust_cm: f64,
    #[serde(default)]
    pub waist_cm: f64,
    #[serde(default)]
    pub hips_cm: f64,
}

/// A built-in size chart.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SizeChart {
    /// European women's sizes 32 to 56 (bust 76–134 cm).
    Women,
    /// European men's sizes 44 to 60 (chest 88–120 cm).
    Men,
}

impl SizeChart {
    /// Name and body of every size. Approximate values from common
    /// European size charts (general garment-making knowledge): women's
    /// sizes grow by 4 cm of bust up to 46, then 6; men's size is half the
    /// chest.
    pub fn sizes(self) -> Vec<(String, Body)> {
        match self {
            SizeChart::Women => (32..=56)
                .step_by(2)
                .map(|n: u32| {
                    let steps = f64::from((n.min(46) - 32) / 2);
                    let big = f64::from(n.saturating_sub(46) / 2);
                    let bust = 76.0 + 4.0 * steps + 6.0 * big;
                    (
                        n.to_string(),
                        Body {
                            masculinity: 0.0,
                            height_cm: 168.0,
                            bust_cm: bust,
                            waist_cm: bust - 18.0,
                            hips_cm: bust + 8.0,
                        },
                    )
                })
                .collect(),
            SizeChart::Men => (44..=60)
                .step_by(2)
                .map(|n: u32| {
                    let chest = 2.0 * f64::from(n);
                    (
                        n.to_string(),
                        Body {
                            masculinity: 1.0,
                            height_cm: 178.0,
                            bust_cm: chest,
                            waist_cm: chest - 12.0,
                            hips_cm: chest + 4.0,
                        },
                    )
                })
                .collect(),
        }
    }
}

/// The project's size range, or an error saying there is none.
pub(crate) fn grading(p: &Project) -> Result<&Grading, CommandError> {
    p.grading()
        .ok_or_else(|| CommandError::invalid("size", Invalid::NoSizes))
}

/// The index of the size called `name`.
pub(crate) fn size_index(p: &Project, name: &str) -> Result<usize, CommandError> {
    grading(p)?
        .index_of(name)
        .ok_or_else(|| CommandError::invalid("size", Invalid::UnknownSize))
}

/// Fill the unknown entries by linear interpolation between known ones,
/// and extrapolation from the last two known on either side.
fn fill(known: &[Option<DVec2>]) -> Vec<DVec2> {
    let at: Vec<(usize, DVec2)> = known
        .iter()
        .enumerate()
        .filter_map(|(k, v)| v.map(|v| (k, v)))
        .collect();
    (0..known.len())
        .map(|k| {
            if let Some(Some(v)) = known.get(k) {
                return *v;
            }
            let left: Vec<_> = at.iter().filter(|(i, _)| *i < k).rev().take(2).collect();
            let right: Vec<_> = at.iter().filter(|(i, _)| *i > k).take(2).collect();
            let line = |(i, a): (usize, DVec2), (j, b): (usize, DVec2)| {
                a + (b - a) * ((k as f64 - i as f64) / (j as f64 - i as f64))
            };
            match (left.first(), right.first(), left.get(1), right.get(1)) {
                (Some(l), Some(r), _, _) => line(**l, **r),
                (Some(l), None, Some(l2), _) => line(**l2, **l),
                (None, Some(r), _, Some(r2)) => line(**r, **r2),
                (Some(only), None, None, _) | (None, Some(only), _, None) => only.1,
                (None, None, ..) => DVec2::ZERO,
            }
        })
        .collect()
}

/// A grade with every size at zero is no grade.
fn normalized(grade: Vec<DVec2>) -> Vec<DVec2> {
    if grade.iter().all(|g| *g == DVec2::ZERO) {
        Vec::new()
    } else {
        grade
    }
}

/// Redraw the pattern in size `i` of the current range (its points,
/// curves, internal lines, pins and avatar), which becomes the base.
fn rebase(tx: &mut Transaction<'_>, i: usize) {
    let at = tx.project().at_size(i);
    let p = tx.project();
    let points: Vec<_> = at
        .points()
        .iter()
        .filter(|(id, pt)| p.point(**id) != Some(*pt))
        .map(|(id, pt)| (*id, pt.clone()))
        .collect();
    let edges: Vec<_> = at
        .edges()
        .iter()
        .filter(|(id, e)| p.edge(**id) != Some(*e))
        .map(|(id, e)| (*id, e.clone()))
        .collect();
    let lines: Vec<_> = at
        .internal_lines()
        .iter()
        .filter(|(id, l)| p.internal_line(**id) != Some(*l))
        .map(|(id, l)| (*id, l.clone()))
        .collect();
    let pins: Vec<_> = at
        .pins()
        .iter()
        .filter(|(id, pin)| p.pins().get(*id) != Some(*pin))
        .map(|(id, pin)| (*id, *pin))
        .collect();
    let avatar = (at.avatar() != p.avatar()).then(|| at.avatar().cloned());
    for (id, pt) in points {
        tx.put_point(id, pt);
    }
    for (id, e) in edges {
        tx.put_edge(id, e);
    }
    for (id, l) in lines {
        tx.put_internal_line(id, l);
    }
    for (id, pin) in pins {
        tx.put_pin(id, pin);
    }
    if let Some(avatar) = avatar {
        tx.set_avatar(avatar);
    }
    tx.set_grading(at.grading().cloned());
}

/// Set the range to `sizes` based on `base`: see [`GradingSetSizes`].
fn set_sizes(
    tx: &mut Transaction<'_>,
    sizes: &[SizeSpec],
    base: &str,
    progress: &Progress,
) -> Result<CommandOutput, CommandError> {
    if sizes.len() < 2 {
        return Err(CommandError::invalid("sizes", Invalid::TooFewSizes));
    }
    let mut names = std::collections::BTreeSet::new();
    for s in sizes {
        non_blank("sizes", &s.name)?;
        if !names.insert(s.name.trim()) {
            return Err(CommandError::invalid("sizes", Invalid::DuplicateName));
        }
        finite("sizes", &[s.height_cm, s.bust_cm, s.waist_cm, s.hips_cm])?;
    }
    let (base_index, zero) = sizes
        .iter()
        .enumerate()
        .find(|(_, s)| s.name.trim() == base.trim())
        .ok_or_else(|| CommandError::invalid("base", Invalid::UnknownSize))?;
    let base_name = zero.name.trim().to_owned();
    let new_sizes: Vec<Size> = sizes
        .iter()
        .map(|s| Size {
            name: s.name.trim().to_owned(),
            height_cm: s.height_cm - zero.height_cm,
            bust_cm: s.bust_cm - zero.bust_cm,
            waist_cm: s.waist_cm - zero.waist_cm,
            hips_cm: s.hips_cm - zero.hips_cm,
        })
        .collect();
    // Redraw the pattern in the new base size, then carry each point's
    // grade over by size name.
    let old_names: Vec<String> = match tx.project().grading().cloned() {
        Some(old) => {
            let i = old
                .index_of(&base_name)
                .ok_or_else(|| CommandError::invalid("base", Invalid::UnknownSize))?;
            rebase(tx, i);
            old.sizes.iter().map(|s| s.name.clone()).collect()
        }
        None => Vec::new(),
    };
    // Every size's body, from the avatar now in the base size.
    if let Some(avatar) = tx.project().avatar() {
        for s in &new_sizes {
            validate_body(&Body {
                height_cm: avatar.body.height_cm + s.height_cm,
                bust_cm: avatar.body.bust_cm + s.bust_cm,
                waist_cm: avatar.body.waist_cm + s.waist_cm,
                hips_cm: avatar.body.hips_cm + s.hips_cm,
                ..avatar.body
            })
            .map_err(|_| CommandError::invalid("sizes", Invalid::BodyOutOfRange))?;
        }
    }
    let shown = tx
        .project()
        .grading()
        .and_then(|g| g.sizes.get(g.shown))
        .and_then(|s| new_sizes.iter().position(|n| n.name == s.name))
        .unwrap_or(base_index);
    let graded: Vec<(PointId, Point)> = tx
        .project()
        .points()
        .iter()
        .filter(|(_, p)| !p.grade.is_empty())
        .map(|(id, p)| (*id, p.clone()))
        .collect();
    tx.set_grading(Some(Grading {
        sizes: new_sizes.clone(),
        base: base_index,
        shown,
    }));
    let old_index = |name: &str| old_names.iter().position(|n| n == name);
    for (id, point) in graded {
        let known: Vec<Option<DVec2>> = new_sizes
            .iter()
            .map(|s| old_index(&s.name).map(|k| point.offset(k)))
            .collect();
        let grade = normalized(fill(&known));
        tx.put_point(id, Point { grade, ..point });
    }
    for (id, edge) in graded_edges(tx.project()) {
        let handle = |h: usize| -> Vec<DVec2> {
            let known: Vec<Option<DVec2>> = new_sizes
                .iter()
                .map(|s| old_index(&s.name).and_then(|k| Some(edge.grade.get(k)?[h])))
                .collect();
            fill(&known)
        };
        let grade: Vec<[DVec2; 2]> = handle(0)
            .into_iter()
            .zip(handle(1))
            .map(|(a, b)| [a, b])
            .collect();
        let grade = if grade.iter().flatten().all(|g| *g == DVec2::ZERO) {
            Vec::new()
        } else {
            grade
        };
        tx.put_edge(id, Edge { grade, ..edge });
    }
    // Garments drafted from a template and not graded yet: drafted again
    // in every size.
    let garments: Vec<GarmentId> = tx
        .project()
        .garments()
        .iter()
        .filter(|(_, g)| g.template.is_some())
        .map(|(id, _)| *id)
        .collect();
    for garment in garments {
        crate::templates::grade_garment(tx, garment, progress)?;
    }
    Ok(CommandOutput::default())
}

/// The edges whose curve handles are graded.
fn graded_edges(p: &Project) -> Vec<(EdgeId, Edge)> {
    p.edges()
        .iter()
        .filter(|(_, e)| !e.grade.is_empty())
        .map(|(id, e)| (*id, e.clone()))
        .collect()
}

/// Set the project's size range.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GradingSetSizes {
    /// From the smallest to the largest; at least two, names unique.
    pub sizes: Vec<SizeSpec>,
    /// The size the pattern is drawn in (a name from `sizes`).
    pub base: String,
}

impl Described for GradingSetSizes {
    const SUMMARY: &'static str = "Set the project's size range: the sizes from smallest to largest, each with how its body differs from the base size's (the avatar; cm), and the base size the pattern is drawn in. Garments drafted from a template and not graded yet are graded by drafting the template for every size's body. Points keep their grade in the sizes that stay (by name); new sizes are inter- or extrapolated. Changing the base redraws the pattern in that size. See grading.use_chart for standard sizes.";
}

impl CommandSpec for GradingSetSizes {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        self.apply_with(tx, &Progress::default())
    }

    /// Grading a template's garment drafts it in every size (seconds).
    fn apply_with(
        &self,
        tx: &mut Transaction<'_>,
        progress: &Progress,
    ) -> Result<CommandOutput, CommandError> {
        set_sizes(tx, &self.sizes, &self.base, progress)
    }
}

/// Set the size range from a built-in chart.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GradingUseChart {
    /// `women` (32–56) or `men` (44–60).
    pub chart: SizeChart,
    /// Smallest size of the range, e.g. "36".
    pub from: String,
    /// Largest size, e.g. "52".
    pub to: String,
    /// The size the pattern is drawn in, e.g. "40".
    pub base: String,
}

impl Described for GradingUseChart {
    const SUMMARY: &'static str = "Set the project's size range from a built-in European chart (`women` 32–56, `men` 44–60, every second size): the sizes from `from` to `to`, drawn in `base`. Each size's body differs from the avatar as the chart's sizes differ from its base size; without an avatar, the chart's base body becomes the avatar. Garments drafted from a template, before or after, are graded to every size; grade other points with point.set_grade.";
}

impl CommandSpec for GradingUseChart {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        self.apply_with(tx, &Progress::default())
    }

    /// Grading a template's garment drafts it in every size (seconds).
    fn apply_with(
        &self,
        tx: &mut Transaction<'_>,
        progress: &Progress,
    ) -> Result<CommandOutput, CommandError> {
        let chart = self.chart.sizes();
        let index = |field: &'static str, name: &str| {
            chart
                .iter()
                .position(|(n, _)| n == name.trim())
                .ok_or_else(|| CommandError::invalid(field, Invalid::UnknownSize))
        };
        let (from, to, base) = (
            index("from", &self.from)?,
            index("to", &self.to)?,
            index("base", &self.base)?,
        );
        if from >= to {
            return Err(CommandError::invalid("to", Invalid::TooFewSizes));
        }
        if !(from..=to).contains(&base) {
            return Err(CommandError::invalid("base", Invalid::UnknownSize));
        }
        let range = chart.get(from..=to).unwrap_or_default();
        let (base_name, base_body) = chart
            .get(base)
            .ok_or_else(|| CommandError::invalid("base", Invalid::UnknownSize))?;
        if tx.project().avatar().is_none() {
            tx.set_avatar(Some(Avatar {
                body: *base_body,
                preset: None,
                mesh: None,
            }));
        }
        let sizes: Vec<SizeSpec> = range
            .iter()
            .map(|(name, b)| SizeSpec {
                name: name.clone(),
                height_cm: b.height_cm - base_body.height_cm,
                bust_cm: b.bust_cm - base_body.bust_cm,
                waist_cm: b.waist_cm - base_body.waist_cm,
                hips_cm: b.hips_cm - base_body.hips_cm,
            })
            .collect();
        set_sizes(tx, &sizes, base_name, progress)
    }
}

/// Remove the size range and every grade.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GradingClear {
    /// Must be `true`: the grades are lost (still undoable).
    #[serde(default)]
    pub confirm: bool,
}

impl Described for GradingClear {
    const SUMMARY: &'static str = "Remove the size range and every point's grade: the pattern stays in its base size. Destructive: requires \"confirm\": true.";
}

impl CommandSpec for GradingClear {
    const DESTRUCTIVE: bool = true;

    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        grading(tx.project())?;
        let graded: Vec<(PointId, Point)> = tx
            .project()
            .points()
            .iter()
            .filter(|(_, p)| !p.grade.is_empty())
            .map(|(id, p)| (*id, p.clone()))
            .collect();
        for (id, point) in graded {
            tx.put_point(
                id,
                Point {
                    grade: Vec::new(),
                    ..point
                },
            );
        }
        for (id, edge) in graded_edges(tx.project()) {
            tx.put_edge(
                id,
                Edge {
                    grade: Vec::new(),
                    ..edge
                },
            );
        }
        tx.set_grading(None);
        Ok(CommandOutput::default())
    }

    fn confirmed(&self) -> bool {
        self.confirm
    }
}

/// Choose the size draped and shown in 3D.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GradingShowSize {
    /// A size of the range.
    pub size: String,
}

impl Described for GradingShowSize {
    const SUMMARY: &'static str = "Choose the size of the range that is draped, shown in 3D and exported in 3D, on an avatar with that size's measurements. The pattern board keeps showing the base size, with the other sizes' outlines around it.";
}

impl CommandSpec for GradingShowSize {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        let shown = size_index(tx.project(), &self.size)?;
        let g = grading(tx.project())?;
        if g.shown != shown {
            let g = Grading { shown, ..g.clone() };
            tx.set_grading(Some(g));
        }
        Ok(CommandOutput::default())
    }
}

/// Set a point's grade rule.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PointSetGrade {
    pub point: PointId,
    /// The same move from each size to the next (mm, [x, y]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub step: Option<Vec2>,
    /// Or where the point is in some sizes, from its base position (mm,
    /// by size name); the other sizes are inter- or extrapolated.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offsets: Option<BTreeMap<String, Vec2>>,
}

impl Described for PointSetGrade {
    const SUMMARY: &'static str = "Set how a contour point moves from size to size (its grade rule): either `step`, the same move from each size to the next (mm, [x, y]; e.g. [5, 0] moves the point 5 mm outward per size), or `offsets`, where it is in some sizes relative to the base size, the others inter- or extrapolated. A zero step removes the rule. Curve handles, internal lines and pins follow the points; linked mirror copies follow their source, mirrored.";
}

impl CommandSpec for PointSetGrade {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        let point = super::editable_point(tx, self.point)?;
        let g = grading(tx.project())?;
        if r3d_core::constraints::formula_graded_points(tx.project()).contains(&self.point) {
            return Err(CommandError::invalid("point", Invalid::GradedByFormula));
        }
        let grade = match (&self.step, &self.offsets) {
            (Some(step), None) => {
                finite("step", step)?;
                let base = g.base as f64;
                (0..g.sizes.len())
                    .map(|k| v(*step) * (k as f64 - base))
                    .collect()
            }
            (None, Some(offsets)) => {
                let mut known = vec![None; g.sizes.len()];
                for (name, off) in offsets {
                    finite("offsets", off)?;
                    let k = g
                        .index_of(name.trim())
                        .ok_or_else(|| CommandError::invalid("offsets", Invalid::UnknownSize))?;
                    if k == g.base && v(*off) != DVec2::ZERO {
                        return Err(CommandError::invalid("offsets", Invalid::BaseOffset));
                    }
                    if let Some(slot) = known.get_mut(k) {
                        *slot = Some(v(*off));
                    }
                }
                if let Some(slot) = known.get_mut(g.base) {
                    *slot = Some(DVec2::ZERO);
                }
                fill(&known)
            }
            _ => return Err(CommandError::invalid("step", Invalid::StepOrOffsets)),
        };
        let grade = normalized(grade);
        if grade != point.grade {
            tx.put_point(self.point, Point { grade, ..point });
        }
        Ok(CommandOutput::default())
    }

    fn merge_key(&self) -> Option<String> {
        Some(format!("point.set_grade:{}", self.point))
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn fill_interpolates_and_extrapolates() {
        let x = |v: f64| Some(DVec2::new(v, 0.0));
        let out = fill(&[None, x(0.0), None, x(4.0), None]);
        let xs: Vec<f64> = out.iter().map(|p| p.x).collect();
        assert_eq!(xs, vec![-2.0, 0.0, 2.0, 4.0, 6.0]);
        assert_eq!(fill(&[None, x(3.0)]), vec![DVec2::new(3.0, 0.0); 2]);
        assert_eq!(fill(&[None, None]), vec![DVec2::ZERO; 2]);
    }

    #[test]
    fn charts_grow_with_the_size() {
        for chart in [SizeChart::Women, SizeChart::Men] {
            let sizes = chart.sizes();
            assert!(sizes.windows(2).all(|w| w[1].1.bust_cm > w[0].1.bust_cm));
            for (_, b) in &sizes {
                validate_body(b).unwrap();
            }
        }
        let women = SizeChart::Women.sizes();
        let bust = |n: &str| women.iter().find(|(s, _)| s == n).unwrap().1.bust_cm;
        assert_eq!((bust("36"), bust("46"), bust("52")), (84.0, 104.0, 122.0));
    }
}
