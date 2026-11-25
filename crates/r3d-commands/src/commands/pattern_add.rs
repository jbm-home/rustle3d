//! Add the garments of a saved pattern (a project document, as the
//! pattern library keeps them) to the project, drafted again for its
//! body (docs/specs/32).

use r3d_core::{AnyId, Garment, GarmentId, Project, SourcePiece, Transaction};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::clip::{Paste, copy, default_offset, paste};
use crate::error::CommandError;
use crate::progress::Progress;
use crate::spec::{CommandOutput, CommandSpec, Described};
use crate::templates::avatar_of;

/// Add the garments of a saved pattern.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PatternAdd {
    /// A Rustle3D project document (`{"format": "rustle3d.project",
    /// "schema_version": N, "project": {...}}`, as `project.json` holds
    /// it), of this version of the app or an older one.
    pub pattern: Value,
}

impl Described for PatternAdd {
    const SUMMARY: &'static str = "Add the garments of a saved pattern (a project document, as library patterns are kept; library.add reads one from the library) to this project: each one with its pieces, seams, notches, internal lines, pins, construction details, buttons, zippers, constraints and variables (a variable the project already has keeps the project's formula), its fabric, colour, print and notes, worn over the garments already there. Pieces drafted from the body are drafted again for this project's avatar, keeping what was done to them since: from a template (the template's body measurements follow the body; lengths and ease stay), from a Seamly2D/Valentina pattern, and dimensions given by formulas; the others come as drawn. In a size range they are graded. Returns the garments, then the panels and seams.";
}

/// Draft a pasted garment, drafted from the body `from`, again for the
/// project's avatar, and grade it in a size range. Pieces whose outline
/// changed since the draft stay as drawn.
fn redraft(
    tx: &mut Transaction<'_>,
    garment: GarmentId,
    from: &r3d_core::Avatar,
    progress: &Progress,
) -> Result<(), CommandError> {
    let Some(g) = tx.project().garments().get(&garment).cloned() else {
        return Ok(());
    };
    // A project without a body takes the pieces as drawn.
    let Some(to) = tx.project().avatar().cloned() else {
        return Ok(());
    };
    let same_body = to.body == from.body && to.measures == from.measures;
    let graded = tx.project().grading().is_some();
    if g.template.is_some() {
        if !same_body {
            crate::templates::redraft(tx, garment, from)?;
        }
        if graded {
            // Drafted for this body: graded from the template again.
            clear_grades(tx, garment);
            crate::templates::grade_garment(tx, garment, progress)?;
        }
    } else if g.source.is_some() {
        if !same_body {
            // Grades it too.
            crate::sources::redraft(tx, garment, to.body, progress)?;
        } else if graded {
            crate::sources::grade(tx, garment, progress)?;
        }
    }
    Ok(())
}

/// Drop the grade rules of `garment`'s pieces.
fn clear_grades(tx: &mut Transaction<'_>, garment: GarmentId) {
    let p = tx.project();
    let ours = |panel| p.panel(panel).is_some_and(|pn| pn.garment == garment);
    let points: Vec<_> = p
        .points()
        .iter()
        .filter(|(_, pt)| ours(pt.panel) && !pt.grade.is_empty())
        .map(|(id, pt)| (*id, pt.clone()))
        .collect();
    let edges: Vec<_> = p
        .edges()
        .iter()
        .filter(|(_, e)| ours(e.panel) && !e.grade.is_empty())
        .map(|(id, e)| (*id, e.clone()))
        .collect();
    for (id, mut pt) in points {
        pt.grade.clear();
        tx.put_point(id, pt);
    }
    for (id, mut e) in edges {
        e.grade.clear();
        tx.put_edge(id, e);
    }
}

impl CommandSpec for PatternAdd {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        self.apply_with(tx, &Progress::default())
    }

    /// Drafting a garment again fits bodies (seconds): reported by size
    /// in a size range.
    fn apply_with(
        &self,
        tx: &mut Transaction<'_>,
        progress: &Progress,
    ) -> Result<CommandOutput, CommandError> {
        let src: Project = r3d_core::file::from_value(self.pattern.clone())?.project;
        let from = avatar_of(&src);
        let mut garments: Vec<AnyId> = Vec::new();
        let mut created: Vec<AnyId> = Vec::new();
        // On an empty board the project's own garment becomes the first
        // one; the others are worn over what is there.
        let p = tx.project();
        let mut reuse = p.panels().is_empty().then(|| p.default_garment()).flatten();
        let mut layer = p
            .garments()
            .values()
            .map(|g| g.layer + 1)
            .max()
            .unwrap_or(0);
        for (old, g) in src.garments() {
            let pieces: Vec<_> = src
                .panels()
                .iter()
                .filter(|(_, pn)| pn.garment == *old)
                .map(|(id, _)| *id)
                .collect();
            if pieces.is_empty() {
                continue;
            }
            let id = match reuse.take() {
                Some(id) => {
                    layer = 0;
                    id
                }
                None => tx.new_garment_id(),
            };
            let clip = copy(&src, &pieces)?;
            let pasted = paste(
                tx,
                &clip,
                &Paste {
                    offset: default_offset(tx.project(), &clip),
                    garment: id,
                    keep_places: true,
                },
            )?;
            let source = g.source.clone().map(|s| r3d_core::PatternSource {
                pieces: s
                    .pieces
                    .iter()
                    .filter_map(|sp| {
                        Some(SourcePiece {
                            panel: *pasted.panels.get(&sp.panel)?,
                            ..*sp
                        })
                    })
                    .collect(),
                ..s
            });
            tx.put_garment(
                id,
                Garment {
                    layer,
                    source,
                    ..g.clone()
                },
            );
            layer += 1;
            garments.push(id.into());
            created.extend(pasted.created);
            redraft(tx, id, &from, progress)?;
        }
        if garments.is_empty() {
            return Err(CommandError::invalid(
                "pattern",
                crate::error::Invalid::Empty,
            ));
        }
        // Variables the project lacks; one it has keeps its formula.
        let mut variables = tx.project().variables().clone();
        for (name, formula) in src.variables() {
            variables
                .entry(name.clone())
                .or_insert_with(|| formula.clone());
        }
        if &variables != tx.project().variables() {
            tx.set_variables(variables);
        }
        tx.project()
            .check_integrity()
            .map_err(|e| CommandError::Malformed {
                name: "pattern".into(),
                detail: e.to_string(),
            })?;
        garments.extend(created);
        Ok(CommandOutput { created: garments })
    }
}
