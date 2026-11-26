//! Constraints and dimensions (spec 19), construction lines, smooth
//! points; and the step that keeps every constraint satisfied.

use r3d_core::Localizable;
use r3d_core::constraints::{
    Touched, Update, formula_grades, formula_value, formula_values, measure, notch_places,
    solve_project,
};
use r3d_core::edit::Change;
use r3d_core::{
    AnyId, Constraint, ConstraintId, Edge, EdgeCurve, EdgeEnd, Guide, GuideId, Notch, Point,
    PointId, Transaction,
};
use r3d_geom::DVec2;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{Vec2, editable_point, v};
use crate::error::{CommandError, Invalid, finite, positive};
use crate::spec::{CommandOutput, CommandSpec, Described};

/// Add a constraint or a dimension.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ConstraintAdd {
    pub constraint: Constraint,
    /// For a dimension: keep the value the drawing has now (the one given
    /// is ignored).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub measure: bool,
}

impl Described for ConstraintAdd {
    const SUMMARY: &'static str = "Add a constraint (fixed, coincident, horizontal, vertical, parallel, perpendicular, tangent, equal_length, on_edge) or a dimension (distance, length, angle) on contour points and edges. A dimension with a `formula` (e.g. \"waist / 4 + 10 mm\") takes its value from the body's measurements and the variables, now and whenever they change, and in every size of the range. The drawing is solved at once: the geometry moves as little as possible so that every constraint holds; constraints that cannot all hold are refused (`constraints_unsolvable`). With `measure`, a dimension keeps the value it has now. Returns the constraint; `constraint.list` shows them with the freedom left.";
}

/// Refuse a point or edge that is missing or derived.
fn check_ref(tx: &Transaction<'_>, id: AnyId) -> Result<(), CommandError> {
    let p = tx.project();
    let derived = match id {
        AnyId::Point(pt) => {
            p.point(pt).ok_or_else(|| CommandError::not_found(pt))?;
            p.is_derived_point(pt)
        }
        AnyId::Edge(e) => {
            p.edge(e).ok_or_else(|| CommandError::not_found(e))?;
            p.is_derived_edge(e)
        }
        _ => false,
    };
    if derived {
        return Err(CommandError::Derived { id });
    }
    Ok(())
}

/// The kind of constraint with its points and edges, ignoring a
/// dimension's value: two constraints with the same key say the same.
fn key(c: &Constraint) -> Option<Constraint> {
    let mut k = c
        .with_value(0.0)
        .and_then(|c| c.with_name(None))
        .and_then(|c| c.with_label(None))
        .unwrap_or_else(|| c.clone());
    // Symmetric relations say the same either way round.
    match &mut k {
        Constraint::Coincident { a, b } => {
            let (x, y) = ((*a).min(*b), (*a).max(*b));
            (*a, *b) = (x, y);
        }
        Constraint::Parallel { a, b }
        | Constraint::Perpendicular { a, b }
        | Constraint::EqualLength { a, b }
        | Constraint::Angle { a, b, .. } => {
            let (x, y) = ((*a).min(*b), (*a).max(*b));
            (*a, *b) = (x, y);
        }
        _ => {}
    }
    Some(k)
}

fn check_value(c: &Constraint) -> Result<(), CommandError> {
    match c {
        Constraint::Distance { mm, .. } | Constraint::Length { mm, .. } => positive("value", *mm),
        Constraint::Angle { deg, .. } => {
            finite("value", &[*deg])?;
            if (0.0..=180.0).contains(deg) {
                Ok(())
            } else {
                Err(CommandError::invalid("value", Invalid::Range(0.0, 180.0)))
            }
        }
        _ => Ok(()),
    }
}

impl CommandSpec for ConstraintAdd {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        let c = &self.constraint;
        for p in c.points() {
            check_ref(tx, p.into())?;
        }
        for e in c.edges() {
            check_ref(tx, e.into())?;
        }
        if let Some(n) = c.notch() {
            let edge = tx
                .project()
                .notch(n)
                .ok_or_else(|| CommandError::not_found(n))?
                .edge;
            check_ref(tx, edge.into())?;
        }
        if let Some(g) = c.guide() {
            tx.project()
                .guide(g)
                .ok_or_else(|| CommandError::not_found(g))?;
        }
        if let Some(name) = c.name() {
            check_name(tx, None, name)?;
        }
        let same = match c {
            Constraint::Coincident { a, b } | Constraint::Distance { a, b, .. } => a == b,
            Constraint::Parallel { a, b }
            | Constraint::Perpendicular { a, b }
            | Constraint::EqualLength { a, b }
            | Constraint::Angle { a, b, .. } => a == b,
            _ => false,
        };
        if same {
            return Err(CommandError::invalid("constraint", Invalid::SamePoint));
        }
        if let Constraint::OnEdge { point, edge } = c
            && tx
                .project()
                .edge(*edge)
                .is_some_and(|e| e.start == *point || e.end == *point)
        {
            // An edge's own end is on it already.
            return Err(CommandError::invalid("constraint", Invalid::Duplicate));
        }
        let c = match formula_value(&tx.project().formula_scope(), c) {
            Some(v) => c
                .with_value(v.map_err(|error| CommandError::Formula {
                    of: "constraint".into(),
                    error,
                })?)
                .unwrap_or_else(|| c.clone()),
            None if self.measure && c.value().is_some() => {
                let now = measure(tx.project(), c).unwrap_or_default();
                c.with_value(now).unwrap_or_else(|| c.clone())
            }
            None => c.clone(),
        };
        check_value(&c)?;
        let k = key(&c);
        if tx.project().constraints().values().any(|o| key(o) == k) {
            return Err(CommandError::invalid("constraint", Invalid::Duplicate));
        }
        let id = tx.new_constraint_id();
        tx.put_constraint(id, c);
        Ok(CommandOutput {
            created: vec![id.into()],
        })
    }
}

/// Remove a constraint.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ConstraintRemove {
    pub constraint: ConstraintId,
}

impl Described for ConstraintRemove {
    const SUMMARY: &'static str =
        "Remove a constraint or a dimension: the geometry stays as it is, free to move again.";
}

impl CommandSpec for ConstraintRemove {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        tx.remove_constraint(self.constraint)
            .ok_or_else(|| CommandError::not_found(self.constraint))?;
        Ok(CommandOutput::default())
    }
}

/// Change a dimension's value.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ConstraintSetValue {
    pub constraint: ConstraintId,
    /// mm for a distance or a length, degrees (0–180) for an angle.
    pub value: f64,
}

impl Described for ConstraintSetValue {
    const SUMMARY: &'static str = "Change a dimension's value (mm, or degrees for an angle): the drawing is redrawn so that it and every other constraint hold, moving as little as possible.";
}

impl CommandSpec for ConstraintSetValue {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        let c = tx
            .project()
            .constraint(self.constraint)
            .ok_or_else(|| CommandError::not_found(self.constraint))?;
        let c = c
            .with_value(self.value)
            .ok_or_else(|| CommandError::invalid("constraint", Invalid::NotDimension))?;
        check_value(&c)?;
        tx.put_constraint(self.constraint, c);
        Ok(CommandOutput::default())
    }

    fn merge_key(&self) -> Option<String> {
        Some(format!("constraint.set_value:{}", self.constraint))
    }
}

/// Refuse a dimension name that formulas could not use, or that a
/// variable or another dimension (than `own`) has.
fn check_name(
    tx: &Transaction<'_>,
    own: Option<ConstraintId>,
    name: &str,
) -> Result<(), CommandError> {
    if !r3d_core::formula::valid_name(name) {
        return Err(CommandError::Formula {
            of: name.to_owned(),
            error: r3d_core::formula::FormulaError::BadName {
                name: name.to_owned(),
            },
        });
    }
    let p = tx.project();
    let taken = p.variables().contains_key(name)
        || p.constraints()
            .iter()
            .any(|(id, c)| Some(*id) != own && c.name() == Some(name));
    if taken {
        return Err(CommandError::invalid("name", Invalid::Duplicate));
    }
    Ok(())
}

/// Name a dimension, so that formulas can use it, or take its name away.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ConstraintRename {
    pub constraint: ConstraintId,
    /// As a variable's: a letter or _, then letters, digits or _ (not a
    /// body measurement, a function, a unit, a variable or another
    /// dimension's name). Omitted: no name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl Described for ConstraintRename {
    const SUMMARY: &'static str = "Name a dimension (e.g. \"armhole\"): formulas can then use it like a variable (its formula when it has one, else its value), e.g. a sleeve cap's length \"armhole + 1.5 cm\". Renaming rewrites the formulas that use the old name; taking the name away is refused while a formula uses it (formula.unknown_name).";
}

/// Whether `text` names `name`.
fn names(text: &str, name: &str) -> bool {
    r3d_core::formula::rename(text, name, "_") != text
}

impl CommandSpec for ConstraintRename {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        let c = tx
            .project()
            .constraint(self.constraint)
            .ok_or_else(|| CommandError::not_found(self.constraint))?
            .clone();
        let name = self
            .name
            .as_deref()
            .map(str::trim)
            .filter(|n| !n.is_empty());
        if let Some(n) = name {
            check_name(tx, Some(self.constraint), n)?;
        }
        let renamed = c
            .with_name(name.map(str::to_owned))
            .ok_or_else(|| CommandError::invalid("constraint", Invalid::NotDimension))?;
        if let Some(old) = c.name()
            && Some(old) != name
        {
            let p = tx.project();
            let users: Vec<(ConstraintId, Constraint)> = p
                .constraints()
                .iter()
                .filter(|(_, o)| o.formula().is_some_and(|f| names(f, old)))
                .map(|(id, o)| (*id, o.clone()))
                .collect();
            let mut variables = p.variables().clone();
            let used_by_variable = variables.values().any(|f| names(f, old));
            match name {
                None if !users.is_empty() || used_by_variable => {
                    return Err(CommandError::Formula {
                        of: self.constraint.to_string(),
                        error: r3d_core::formula::FormulaError::UnknownName {
                            name: old.to_owned(),
                        },
                    });
                }
                None => {}
                Some(new) => {
                    for (id, o) in users {
                        let f = o.formula().map(|f| r3d_core::formula::rename(f, old, new));
                        if let Some(o) = o.with_formula(f) {
                            tx.put_constraint(id, o);
                        }
                    }
                    if used_by_variable {
                        for f in variables.values_mut() {
                            *f = r3d_core::formula::rename(f, old, new);
                        }
                        tx.set_variables(variables);
                    }
                }
            }
        }
        // A formula renamed with it is read again from the new name.
        let renamed = match tx.project().constraint(self.constraint) {
            Some(now) => now.with_name(name.map(str::to_owned)).unwrap_or(renamed),
            None => renamed,
        };
        tx.put_constraint(self.constraint, renamed);
        Ok(CommandOutput::default())
    }
}

/// Place a dimension's label by hand.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ConstraintPlaceLabel {
    pub constraint: ConstraintId,
    /// mm from where the label is drawn by default (the middle of the
    /// dimension line, of the edge, of the angle). Omitted: back there.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offset: Option<Vec2>,
}

impl Described for ConstraintPlaceLabel {
    const SUMMARY: &'static str = "Move a dimension's label (only the label: the drawing stays as it is), as an offset in mm from where it is drawn by default; without `offset`, back there.";
}

impl CommandSpec for ConstraintPlaceLabel {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        if let Some(o) = &self.offset {
            finite("offset", o)?;
        }
        let c = tx
            .project()
            .constraint(self.constraint)
            .ok_or_else(|| CommandError::not_found(self.constraint))?
            .with_label(self.offset)
            .ok_or_else(|| CommandError::invalid("constraint", Invalid::NotDimension))?;
        tx.put_constraint(self.constraint, c);
        Ok(CommandOutput::default())
    }

    fn merge_key(&self) -> Option<String> {
        Some(format!("constraint.place_label:{}", self.constraint))
    }
}

/// Give a dimension a formula, or take it away.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ConstraintSetFormula {
    pub constraint: ConstraintId,
    /// E.g. "waist / 4 + 10 mm", "bust / 2 + ease": arithmetic (+ - * /,
    /// parentheses, min, max, abs, sqrt) on numbers with units (mm, cm, m,
    /// in, deg), the body's measurements (height, bust, waist, hips) and
    /// variables. A length for a distance or a length, a plain number
    /// (degrees) for an angle. Omitted: the dimension keeps its value and
    /// no longer follows the body.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub formula: Option<String>,
}

impl Described for ConstraintSetFormula {
    const SUMMARY: &'static str = "Give a dimension a formula of the body's measurements and the variables (e.g. \"waist / 4 + 10 mm\"): its value follows them, the drawing is redrawn when the body changes, and each size of the range is drafted with its own measurements (the points the formulas move are graded by them). Without `formula`, the dimension keeps its value. Errors say what is wrong: formula.syntax, formula.unknown_name, formula.cycle, formula.unit, formula.no_body, formula.domain, formula.range.";
}

impl CommandSpec for ConstraintSetFormula {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        let c = tx
            .project()
            .constraint(self.constraint)
            .ok_or_else(|| CommandError::not_found(self.constraint))?;
        let formula = self
            .formula
            .as_deref()
            .map(str::trim)
            .filter(|f| !f.is_empty())
            .map(str::to_owned);
        let c = c
            .with_formula(formula)
            .ok_or_else(|| CommandError::invalid("constraint", Invalid::NotDimension))?;
        let c = match formula_value(&tx.project().formula_scope(), &c) {
            Some(v) => c
                .with_value(v.map_err(|error| CommandError::Formula {
                    of: self.constraint.to_string(),
                    error,
                })?)
                .unwrap_or(c),
            None => c,
        };
        tx.put_constraint(self.constraint, c);
        Ok(CommandOutput::default())
    }
}

/// Set a named variable for formulas.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct VariableSet {
    /// A letter or _, then letters, digits or _ (not a body measurement,
    /// a function or a unit).
    pub name: String,
    /// Its formula, e.g. "2 cm" or "bust / 20 + 5 mm".
    pub formula: String,
}

impl Described for VariableSet {
    const SUMMARY: &'static str = "Set a named variable that formulas can use (e.g. ease = \"2 cm\", dart = \"(hips - waist) / 8\"): created, or changed, and every formula using it follows.";
}

impl CommandSpec for VariableSet {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        let name = self.name.trim();
        if !r3d_core::formula::valid_name(name) {
            return Err(CommandError::Formula {
                of: name.to_owned(),
                error: r3d_core::formula::FormulaError::BadName {
                    name: name.to_owned(),
                },
            });
        }
        if tx.project().formula_scope().dimension(name).is_some() {
            return Err(CommandError::invalid("name", Invalid::Duplicate));
        }
        let mut variables = tx.project().variables().clone();
        variables.insert(name.to_owned(), self.formula.trim().to_owned());
        // It must compute, and so must those using it.
        check_variables(tx, &variables, Some(name))?;
        tx.set_variables(variables);
        Ok(CommandOutput::default())
    }

    fn merge_key(&self) -> Option<String> {
        Some(format!("variable.set:{}", self.name.trim()))
    }
}

/// Remove a named variable.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct VariableRemove {
    pub name: String,
}

impl Described for VariableRemove {
    const SUMMARY: &'static str =
        "Remove a named variable; refused while a formula uses it (formula.unknown_name).";
}

impl CommandSpec for VariableRemove {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        let mut variables = tx.project().variables().clone();
        if variables.remove(self.name.trim()).is_none() {
            return Err(CommandError::invalid("name", Invalid::Empty));
        }
        // Still used by another variable: say which.
        check_variables(tx, &variables, None)?;
        tx.set_variables(variables);
        Ok(CommandOutput::default())
    }
}

/// Refuse `variables` if one of them does not compute (one using a body
/// measurement without a body waits for it); `first` is checked first,
/// so that its own error is the one reported.
fn check_variables(
    tx: &Transaction<'_>,
    variables: &std::collections::BTreeMap<String, String>,
    first: Option<&str>,
) -> Result<(), CommandError> {
    let scope = r3d_core::formula::Scope {
        body: tx.project().avatar(),
        variables,
        dimensions: tx.project().constraints(),
    };
    for name in first
        .into_iter()
        .chain(variables.keys().map(String::as_str))
    {
        match scope.eval(name) {
            Ok(_) | Err(r3d_core::formula::FormulaError::NoBody { .. }) => {}
            Err(error) => {
                return Err(CommandError::Formula {
                    of: name.to_owned(),
                    error,
                });
            }
        }
    }
    Ok(())
}

/// The named variables, with their values.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct VariableList {}

impl Described for VariableList {
    const SUMMARY: &'static str = "List the named variables, each with its formula and its value in the size drawn (`mm` for a length, `number` otherwise; none while it needs a body the project does not have), and the body's measurements formulas can use (mm: the four and the others, typed or estimated).";
}

impl crate::spec::QuerySpec for VariableList {
    fn run(&self, p: &r3d_core::Project) -> Result<serde_json::Value, CommandError> {
        let scope = p.formula_scope();
        let round = |x: f64| (x * 1000.0).round() / 1000.0;
        let variables: Vec<serde_json::Value> = p
            .variables()
            .iter()
            .map(|(name, formula)| {
                let mut out = serde_json::json!({ "name": name, "formula": formula });
                match scope.eval(name) {
                    Ok(q) if q.power == 1 => out["mm"] = serde_json::json!(round(q.value)),
                    Ok(q) if q.power == 0 => out["number"] = serde_json::json!(round(q.value)),
                    Ok(_) => {}
                    Err(e) => out["error"] = serde_json::json!(e.message().code),
                }
                out
            })
            .collect();
        let body = p.avatar().map(|a| {
            let mut body = serde_json::json!({
                "height": a.body.height_cm * 10.0,
                "bust": a.body.bust_cm * 10.0,
                "waist": a.body.waist_cm * 10.0,
                "hips": a.body.hips_cm * 10.0,
            });
            for m in r3d_core::Measure::ALL {
                body[m.name()] = (a.measure(m).cm * 10.0).into();
            }
            body
        });
        Ok(serde_json::json!({ "variables": variables, "body": body }))
    }
}

/// Make a point smooth or a corner.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PointSetSmooth {
    pub point: PointId,
    /// `true`: the outline passes through the point without a corner (its
    /// two edges become curves with aligned handles, kept aligned by a
    /// `tangent` constraint); `false`: a corner again (the constraint goes,
    /// the shape stays).
    pub smooth: bool,
}

impl Described for PointSetSmooth {
    const SUMMARY: &'static str = "Make a contour point smooth (no corner: its two edges become curves whose handles line up, and stay so through a `tangent` constraint) or a corner again (the constraint is removed, the shape stays).";
}

impl CommandSpec for PointSetSmooth {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        let point = editable_point(tx, self.point)?;
        let tangents: Vec<ConstraintId> = tx
            .project()
            .constraints()
            .iter()
            .filter(|(_, c)| matches!(c, Constraint::Tangent { point } if *point == self.point))
            .map(|(id, _)| *id)
            .collect();
        if !self.smooth {
            for id in tangents {
                tx.remove_constraint(id);
            }
            return Ok(CommandOutput::default());
        }
        if !tangents.is_empty() {
            return Ok(CommandOutput::default());
        }
        let p = tx.project();
        let panel = p
            .panel(point.panel)
            .ok_or_else(|| CommandError::not_found(point.panel))?;
        let find = |end: bool| {
            panel.contour.iter().find_map(|id| {
                let e = p.edge(*id)?;
                let hit = if end { e.end } else { e.start } == self.point;
                hit.then(|| (*id, e.clone()))
            })
        };
        let (Some((arriving, ein)), Some((leaving, eout))) = (find(true), find(false)) else {
            return Err(CommandError::invalid("point", Invalid::NotJoined));
        };
        let pos = |id: PointId| p.point(id).map(|p| p.pos).unwrap_or_default();
        let at = point.pos;
        // Handles: the edges' own, or a third of the way along a line.
        let (c1_in, c2_in) = match ein.curve {
            EdgeCurve::Cubic { c1, c2 } => (c1, c2),
            EdgeCurve::Line => {
                let a = pos(ein.start);
                (a + (at - a) / 3.0, a + (at - a) * 2.0 / 3.0)
            }
        };
        let (c1_out, c2_out) = match eout.curve {
            EdgeCurve::Cubic { c1, c2 } => (c1, c2),
            EdgeCurve::Line => {
                let b = pos(eout.end);
                (at + (b - at) / 3.0, at + (b - at) * 2.0 / 3.0)
            }
        };
        // One direction through the point: between the arriving and the
        // leaving directions.
        let u_in = (at - c2_in).try_normalize().unwrap_or(DVec2::X);
        let u_out = (c1_out - at).try_normalize().unwrap_or(DVec2::X);
        let u = (u_in + u_out).try_normalize().unwrap_or(u_out);
        let c2_in = at - u * (at - c2_in).length();
        let c1_out = at + u * (c1_out - at).length();
        tx.put_edge(
            arriving,
            Edge {
                curve: EdgeCurve::Cubic {
                    c1: c1_in,
                    c2: c2_in,
                },
                ..ein
            },
        );
        tx.put_edge(
            leaving,
            Edge {
                curve: EdgeCurve::Cubic {
                    c1: c1_out,
                    c2: c2_out,
                },
                ..eout
            },
        );
        let id = tx.new_constraint_id();
        tx.put_constraint(id, Constraint::Tangent { point: self.point });
        Ok(CommandOutput {
            created: vec![id.into()],
        })
    }
}

/// Add a construction line.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GuideAdd {
    /// One end on the board (mm).
    pub a: Vec2,
    /// The other end (mm).
    pub b: Vec2,
}

impl Described for GuideAdd {
    const SUMMARY: &'static str = "Add a construction line on the board, from `a` to `b` (mm): drawn to draft against and snapped to, never cut, sewn or exported. Returns the line.";
}

fn guide(a: Vec2, b: Vec2) -> Result<Guide, CommandError> {
    finite("a", &a)?;
    finite("b", &b)?;
    if a == b {
        return Err(CommandError::invalid("b", Invalid::SamePoint));
    }
    Ok(Guide { a: v(a), b: v(b) })
}

impl CommandSpec for GuideAdd {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        let g = guide(self.a, self.b)?;
        let id = tx.new_guide_id();
        tx.put_guide(id, g);
        Ok(CommandOutput {
            created: vec![id.into()],
        })
    }
}

/// Move a construction line's ends.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GuideMove {
    pub guide: GuideId,
    pub a: Vec2,
    pub b: Vec2,
}

impl Described for GuideMove {
    const SUMMARY: &'static str = "Move a construction line: its new ends `a` and `b` (mm).";
}

impl CommandSpec for GuideMove {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        tx.project()
            .guides()
            .get(&self.guide)
            .ok_or_else(|| CommandError::not_found(self.guide))?;
        let g = guide(self.a, self.b)?;
        tx.put_guide(self.guide, g);
        Ok(CommandOutput::default())
    }

    fn merge_key(&self) -> Option<String> {
        Some(format!("guide.move:{}", self.guide))
    }
}

/// Remove a construction line.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GuideRemove {
    pub guide: GuideId,
}

impl Described for GuideRemove {
    const SUMMARY: &'static str = "Remove a construction line.";
}

impl CommandSpec for GuideRemove {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        tx.remove_guide(self.guide)
            .ok_or_else(|| CommandError::not_found(self.guide))?;
        Ok(CommandOutput::default())
    }
}

/// What the step's changes moved: the points and curves to hold.
fn touched(changes: &[Change]) -> Touched {
    let mut t = Touched::default();
    for c in changes {
        match c {
            Change::Point(c) => {
                if let (Some(b), Some(a)) = (&c.before, &c.after)
                    && b.pos != a.pos
                {
                    t.points.insert(c.id);
                }
            }
            Change::Edge(c) => {
                if let (Some(b), Some(a)) = (&c.before, &c.after)
                    && b.curve != a.curve
                {
                    t.edges.insert(c.id);
                }
            }
            _ => {}
        }
    }
    t
}

/// After a step: drop the constraints whose points or edges are gone (or
/// now derived), then solve, so that every constraint holds again. The
/// changes join the step's undo step.
pub(crate) fn settle_constraints(tx: &mut Transaction<'_>) -> Result<(), CommandError> {
    let p = tx.project();
    let orphans: Vec<ConstraintId> = p
        .constraints()
        .iter()
        .filter(|(_, c)| {
            c.points()
                .iter()
                .any(|pt| p.point(*pt).is_none() || p.is_derived_point(*pt))
                || c.edges()
                    .iter()
                    .any(|e| p.edge(*e).is_none() || p.is_derived_edge(*e))
                || c.notch().is_some_and(|n| {
                    p.notch(n)
                        .is_none_or(|n| p.edge(n.edge).is_none() || p.is_derived_edge(n.edge))
                })
                || c.guide().is_some_and(|g| p.guide(g).is_none())
        })
        .map(|(id, _)| *id)
        .collect();
    for id in orphans {
        tx.remove_constraint(id);
    }
    if tx.project().constraints().is_empty() {
        return Ok(());
    }
    // Formulas first: the body or a variable may have changed.
    let values = formula_values(tx.project()).map_err(|(id, error)| CommandError::Formula {
        of: id.to_string(),
        error,
    })?;
    for (id, v) in values {
        if let Some(c) = tx.project().constraint(id).and_then(|c| c.with_value(v)) {
            tx.put_constraint(id, c);
        }
    }
    let touched = touched(tx.changes());
    let updates = solve_project(tx.project(), &touched).map_err(|e| CommandError::Unsolvable {
        constraints: e.constraints.into_iter().map(AnyId::from).collect(),
    })?;
    for u in updates {
        match u {
            Update::Point(id, to) => {
                if let Some(pt) = tx.project().point(id).cloned() {
                    tx.put_point(id, Point { pos: to, ..pt });
                }
            }
            Update::Curve(id, curve) => {
                if let Some(e) = tx.project().edge(id).cloned() {
                    tx.put_edge(id, Edge { curve, ..e });
                }
            }
        }
    }
    // Notches where their dimensions put them on the edges as they now are.
    for (id, offset_mm) in notch_places(tx.project()) {
        if let Some(n) = tx.project().notch(id).cloned() {
            tx.put_notch(id, Notch { offset_mm, ..n });
        }
    }
    // Then every size of the range, drafted with its own measurements.
    let unsolvable = |e: r3d_core::constraints::Unsolvable| CommandError::Unsolvable {
        constraints: e.constraints.into_iter().map(AnyId::from).collect(),
    };
    let grades = formula_grades(tx.project()).map_err(unsolvable)?;
    for (id, grade) in grades.points {
        if let Some(pt) = tx.project().point(id).cloned()
            && pt.grade != grade
        {
            tx.put_point(id, Point { grade, ..pt });
        }
    }
    for (id, grade) in grades.curves {
        if let Some(e) = tx.project().edge(id).cloned()
            && e.grade != grade
        {
            tx.put_edge(id, Edge { grade, ..e });
        }
    }
    Ok(())
}

/// Remove the constraints naming any of `edges` (for edits that replace an
/// edge by others, so that what was said of it no longer applies).
pub(crate) fn drop_constraints_on(tx: &mut Transaction<'_>, edges: &[r3d_core::EdgeId]) {
    let gone: Vec<ConstraintId> = tx
        .project()
        .constraints()
        .iter()
        .filter(|(_, c)| c.edges().iter().any(|e| edges.contains(e)))
        .map(|(id, _)| *id)
        .collect();
    for id in gone {
        tx.remove_constraint(id);
    }
}

/// The constraints and dimensions, with how constrained the drawing is.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ConstraintList {
    /// Only those on this piece. Omitted: all of them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub panel: Option<r3d_core::PanelId>,
}

impl Described for ConstraintList {
    const SUMMARY: &'static str = "List the constraints and dimensions (on one piece, or all): each with its piece, its kind and what it names, a dimension's value and what the drawing measures now, whether it holds, and `marks` (where its points and the middles of its edges are, mm). `groups` are constraints sharing points, each with `freedom` (ways the constrained geometry can still move, moving or turning a piece as a whole included) and `redundant` constraints (over-constrained: they repeat what others say).";
}

impl crate::spec::QuerySpec for ConstraintList {
    fn run(&self, p: &r3d_core::Project) -> Result<serde_json::Value, CommandError> {
        if let Some(panel) = self.panel {
            p.panel(panel)
                .ok_or_else(|| CommandError::not_found(panel))?;
        }
        let on_panel = |c: &Constraint| {
            self.panel.is_none_or(|panel| {
                c.points()
                    .iter()
                    .any(|pt| p.point(*pt).is_some_and(|pt| pt.panel == panel))
                    || c.edges()
                        .iter()
                        .any(|e| p.edge(*e).is_some_and(|e| e.panel == panel))
                    || p.constraint_panel(c) == Some(panel)
            })
        };
        let round = |x: f64| (x * 1000.0).round() / 1000.0;
        let constraints: Vec<serde_json::Value> = p
            .constraints()
            .iter()
            .filter(|(_, c)| on_panel(c))
            .map(|(id, c)| {
                let mut out = serde_json::to_value(c).unwrap_or_default();
                if let Some(obj) = out.as_object_mut() {
                    obj.insert("id".into(), serde_json::json!(id));
                    obj.insert("panel".into(), serde_json::json!(p.constraint_panel(c)));
                    obj.insert(
                        "holds".into(),
                        serde_json::json!(r3d_core::constraints::holds(p, *id)),
                    );
                    if let Some(now) = measure(p, c) {
                        obj.insert("measured".into(), serde_json::json!(round(now)));
                    }
                    // Where to draw it: the points it names, the middles
                    // of its edges.
                    let mut marks: Vec<[f64; 2]> = c
                        .points()
                        .iter()
                        .filter_map(|pt| p.point(*pt))
                        .map(|pt| [round(pt.pos.x), round(pt.pos.y)])
                        .collect();
                    for e in c.edges() {
                        if let Some(g) = p.edge_geometry(e) {
                            let (at, _) = g.point_at_length(g.length() / 2.0);
                            marks.push([round(at.x), round(at.y)]);
                        }
                    }
                    marks.extend(
                        extra_marks(p, c)
                            .into_iter()
                            .map(|at| [round(at.x), round(at.y)]),
                    );
                    obj.insert("marks".into(), serde_json::json!(marks));
                }
                out
            })
            .collect();
        let groups: Vec<serde_json::Value> = r3d_core::constraints::status(p)
            .into_iter()
            .filter(|g| {
                g.constraints
                    .iter()
                    .any(|id| p.constraint(*id).is_some_and(on_panel))
            })
            .map(|g| {
                serde_json::json!({
                    "constraints": g.constraints,
                    "freedom": g.freedom,
                    "redundant": g.redundant,
                    "satisfied": g.satisfied,
                })
            })
            .collect();
        Ok(serde_json::json!({ "constraints": constraints, "groups": groups }))
    }
}

/// Where to draw what a constraint names besides its points and edges:
/// a notch dimension's edge end and notch, the foot of a point's
/// perpendicular on a construction line.
fn extra_marks(p: &r3d_core::Project, c: &Constraint) -> Vec<DVec2> {
    match c {
        Constraint::NotchDistance { notch, from, .. } => {
            let Some(n) = p.notch(*notch) else {
                return Vec::new();
            };
            let Some(edge) = p.edge(n.edge) else {
                return Vec::new();
            };
            let end = match from {
                EdgeEnd::Start => edge.start,
                EdgeEnd::End => edge.end,
            };
            let at = p.edge_geometry(n.edge).map(|g| {
                let length = g.length();
                g.point_at_length(n.offset_mm.min(length)).0
            });
            p.point(end)
                .map(|pt| pt.pos)
                .into_iter()
                .chain(at)
                .collect()
        }
        Constraint::OnGuide { point, guide } | Constraint::GuideDistance { point, guide, .. } => {
            match (p.point(*point), p.guide(*guide)) {
                (Some(pt), Some(g)) => {
                    let dir = (g.b - g.a).normalize_or_zero();
                    vec![g.a + dir * dir.dot(pt.pos - g.a)]
                }
                _ => Vec::new(),
            }
        }
        _ => Vec::new(),
    }
}

/// The construction lines.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GuideList {}

impl Described for GuideList {
    const SUMMARY: &'static str =
        "List the construction lines: each with its ends `a` and `b` (mm).";
}

impl crate::spec::QuerySpec for GuideList {
    fn run(&self, p: &r3d_core::Project) -> Result<serde_json::Value, CommandError> {
        Ok(serde_json::Value::Array(
            p.guides()
                .iter()
                .map(|(id, g)| serde_json::json!({ "id": id, "a": [g.a.x, g.a.y], "b": [g.b.x, g.b.y] }))
                .collect(),
        ))
    }
}
