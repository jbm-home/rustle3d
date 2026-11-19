//! Recorded edits: the only way to modify a [`Project`].
//!
//! A [`Transaction`] applies changes immediately and records each one as
//! `(id, before, after)`. Committing returns a [`ChangeSet`] that can be
//! undone and redone generically; dropping an uncommitted transaction rolls
//! it back, so a failing command can never leave a half-applied document.

use std::collections::BTreeMap;
use std::mem;

use crate::id::{
    ButtonId, ConstraintId, EdgeId, GarmentId, GuideId, InternalLineId, NotchId, PanelId, PinId,
    PointId, SeamId, ZipperId,
};
use crate::model::{
    Avatar, Constraint, DisplayUnit, Edge, Garment, Grading, Guide, IdMap, InternalLine, Notch,
    Panel, Pin, Point, Project, Seam,
};
use crate::trim::{Button, Zipper};

/// Before/after values of one entity. `None` means "absent".
#[derive(Clone, Debug, PartialEq)]
pub struct EntityChange<K, V> {
    pub id: K,
    pub before: Option<V>,
    pub after: Option<V>,
}

/// One recorded change. Entity changes are boxed so that the enum stays
/// small whatever the size of the entities.
#[derive(Clone, Debug, PartialEq)]
pub enum Change {
    Garment(Box<EntityChange<GarmentId, Garment>>),
    Panel(Box<EntityChange<PanelId, Panel>>),
    Point(Box<EntityChange<PointId, Point>>),
    Edge(Box<EntityChange<EdgeId, Edge>>),
    Notch(Box<EntityChange<NotchId, Notch>>),
    InternalLine(Box<EntityChange<InternalLineId, InternalLine>>),
    Seam(Box<EntityChange<SeamId, Seam>>),
    Pin(Box<EntityChange<PinId, Pin>>),
    Constraint(Box<EntityChange<ConstraintId, Constraint>>),
    Guide(Box<EntityChange<GuideId, Guide>>),
    Button(Box<EntityChange<ButtonId, Button>>),
    Zipper(Box<EntityChange<ZipperId, Zipper>>),
    Avatar {
        before: Option<Avatar>,
        after: Option<Avatar>,
    },
    Name {
        before: String,
        after: String,
    },
    DisplayUnit {
        before: DisplayUnit,
        after: DisplayUnit,
    },
    Grading {
        before: Option<Grading>,
        after: Option<Grading>,
    },
    Variables {
        before: BTreeMap<String, String>,
        after: BTreeMap<String, String>,
    },
}

fn set<K: Ord + Copy, V: Clone>(map: &mut IdMap<K, V>, id: K, value: &Option<V>) {
    match value {
        Some(v) => {
            map.insert(id, v.clone());
        }
        None => {
            map.remove(&id);
        }
    }
}

impl Change {
    fn apply(&self, p: &mut Project, forward: bool) {
        macro_rules! pick {
            ($c:expr) => {
                if forward { &$c.after } else { &$c.before }
            };
        }
        match self {
            Change::Garment(c) => set(&mut p.garments, c.id, pick!(c)),
            Change::Panel(c) => set(&mut p.panels, c.id, pick!(c)),
            Change::Point(c) => set(&mut p.points, c.id, pick!(c)),
            Change::Edge(c) => set(&mut p.edges, c.id, pick!(c)),
            Change::Notch(c) => set(&mut p.notches, c.id, pick!(c)),
            Change::InternalLine(c) => set(&mut p.internal_lines, c.id, pick!(c)),
            Change::Seam(c) => set(&mut p.seams, c.id, pick!(c)),
            Change::Pin(c) => set(&mut p.pins, c.id, pick!(c)),
            Change::Constraint(c) => set(&mut p.constraints, c.id, pick!(c)),
            Change::Guide(c) => set(&mut p.guides, c.id, pick!(c)),
            Change::Button(c) => set(&mut p.buttons, c.id, pick!(c)),
            Change::Zipper(c) => set(&mut p.zippers, c.id, pick!(c)),
            Change::Name { before, after } => {
                p.name = if forward { after } else { before }.clone();
            }
            Change::DisplayUnit { before, after } => {
                p.display_unit = if forward { *after } else { *before };
            }
            Change::Avatar { before, after } => {
                p.avatar = if forward { after } else { before }.clone();
            }
            Change::Grading { before, after } => {
                p.grading = if forward { after } else { before }.clone();
            }
            Change::Variables { before, after } => {
                p.variables = if forward { after } else { before }.clone();
            }
        }
    }

    /// If `self` and `later` touch the same thing, fold `later` into `self`
    /// (keep the first `before`, take the last `after`) and return `true`.
    fn absorb(&mut self, later: &Change) -> bool {
        match (self, later) {
            (Change::Garment(a), Change::Garment(b)) if a.id == b.id => a.after = b.after.clone(),
            (Change::Panel(a), Change::Panel(b)) if a.id == b.id => a.after = b.after.clone(),
            (Change::Point(a), Change::Point(b)) if a.id == b.id => a.after = b.after.clone(),
            (Change::Edge(a), Change::Edge(b)) if a.id == b.id => a.after = b.after.clone(),
            (Change::Notch(a), Change::Notch(b)) if a.id == b.id => a.after = b.after.clone(),
            (Change::Seam(a), Change::Seam(b)) if a.id == b.id => a.after = b.after.clone(),
            (Change::Pin(a), Change::Pin(b)) if a.id == b.id => a.after = b.after,
            (Change::Constraint(a), Change::Constraint(b)) if a.id == b.id => {
                a.after = b.after.clone()
            }
            (Change::Guide(a), Change::Guide(b)) if a.id == b.id => a.after = b.after.clone(),
            (Change::Button(a), Change::Button(b)) if a.id == b.id => a.after = b.after.clone(),
            (Change::Zipper(a), Change::Zipper(b)) if a.id == b.id => a.after = b.after.clone(),
            (Change::InternalLine(a), Change::InternalLine(b)) if a.id == b.id => {
                a.after = b.after.clone()
            }
            (Change::Name { after, .. }, Change::Name { after: b, .. }) => *after = b.clone(),
            (Change::Avatar { after, .. }, Change::Avatar { after: b, .. }) => *after = b.clone(),
            (Change::DisplayUnit { after, .. }, Change::DisplayUnit { after: b, .. }) => {
                *after = *b
            }
            (Change::Grading { after, .. }, Change::Grading { after: b, .. }) => *after = b.clone(),
            (Change::Variables { after, .. }, Change::Variables { after: b, .. }) => {
                *after = b.clone()
            }
            _ => return false,
        }
        true
    }
}

/// The recorded result of a committed transaction.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ChangeSet {
    pub changes: Vec<Change>,
    pub next_id_before: u64,
    pub next_id_after: u64,
}

impl ChangeSet {
    pub fn is_empty(&self) -> bool {
        self.changes.is_empty() && self.next_id_before == self.next_id_after
    }

    /// Revert these changes on `project`.
    pub fn undo(&self, project: &mut Project) {
        for c in self.changes.iter().rev() {
            c.apply(project, false);
        }
        project.next_id = self.next_id_before;
    }

    /// Re-apply these changes on `project`.
    pub fn redo(&self, project: &mut Project) {
        for c in &self.changes {
            c.apply(project, true);
        }
        project.next_id = self.next_id_after;
    }

    /// The panels these changes touch that exist in `project`, in the
    /// order first touched: what to select after undoing or redoing them,
    /// so the user sees what changed. A notch or a seam counts for the
    /// panels of its edges.
    pub fn panels(&self, project: &Project) -> Vec<PanelId> {
        let edge_panel = |e: EdgeId| project.edge(e).map(|e| e.panel);
        let mut out: Vec<PanelId> = Vec::new();
        let mut add = |id: Option<PanelId>| {
            if let Some(id) = id
                && project.panel(id).is_some()
                && !out.contains(&id)
            {
                out.push(id);
            }
        };
        for c in &self.changes {
            macro_rules! each {
                ($c:expr, $f:expr) => {
                    for v in [&$c.before, &$c.after].into_iter().flatten() {
                        for id in $f(v) {
                            add(id);
                        }
                    }
                };
            }
            match c {
                Change::Panel(c) => add(Some(c.id)),
                Change::Point(c) => each!(c, |v: &Point| [Some(v.panel)]),
                Change::Edge(c) => each!(c, |v: &Edge| [Some(v.panel)]),
                Change::InternalLine(c) => each!(c, |v: &InternalLine| [Some(v.panel)]),
                Change::Pin(c) => each!(c, |v: &Pin| [Some(v.panel)]),
                Change::Notch(c) => each!(c, |v: &Notch| [edge_panel(v.edge)]),
                Change::Seam(c) => {
                    each!(c, |v: &Seam| [
                        project.side_panel(&v.a),
                        project.side_panel(&v.b)
                    ])
                }
                Change::Constraint(c) => {
                    each!(c, |v: &Constraint| [project.constraint_panel(v)])
                }
                Change::Button(c) => each!(c, |v: &Button| [Some(v.panel), Some(v.hole.panel)]),
                Change::Zipper(c) => {
                    each!(c, |v: &Zipper| [
                        project.side_panel(&v.a),
                        project.side_panel(&v.b)
                    ])
                }
                Change::Garment(_)
                | Change::Guide(_)
                | Change::Avatar { .. }
                | Change::Name { .. }
                | Change::DisplayUnit { .. }
                | Change::Grading { .. }
                | Change::Variables { .. } => {}
            }
        }
        out
    }

    /// Merge a later change set into this one, compacting per entity so that
    /// e.g. dragging a point for 300 frames stays one small undo step.
    ///
    /// Compaction is valid because entity changes are absolute values: the
    /// result only depends on the first `before` and the last `after`.
    pub fn append(&mut self, later: ChangeSet) {
        for change in later.changes {
            if !self.changes.iter_mut().any(|c| c.absorb(&change)) {
                self.changes.push(change);
            }
        }
        self.next_id_after = later.next_id_after;
    }
}

/// An open edit on a project. See the module docs.
#[derive(Debug)]
pub struct Transaction<'a> {
    project: &'a mut Project,
    changes: Vec<Change>,
    next_id_before: u64,
    committed: bool,
}

macro_rules! entity_ops {
    ($kind:literal, $Id:ident, $V:ident, $map:ident, $variant:ident, $new_id:ident, $put:ident, $remove:ident) => {
        #[doc = concat!("Allocate a fresh ", $kind, " ID.")]
        pub fn $new_id(&mut self) -> $Id {
            $Id::from_raw(self.alloc())
        }

        #[doc = concat!("Insert or replace a ", $kind, ".")]
        pub fn $put(&mut self, id: $Id, value: $V) {
            let before = self.project.$map.insert(id, value.clone());
            self.changes.push(Change::$variant(Box::new(EntityChange {
                id,
                before,
                after: Some(value),
            })));
        }

        #[doc = concat!("Remove a ", $kind, "; returns it if it existed.")]
        pub fn $remove(&mut self, id: $Id) -> Option<$V> {
            let before = self.project.$map.remove(&id)?;
            self.changes.push(Change::$variant(Box::new(EntityChange {
                id,
                before: Some(before.clone()),
                after: None,
            })));
            Some(before)
        }
    };
}

impl<'a> Transaction<'a> {
    pub fn begin(project: &'a mut Project) -> Self {
        let next_id_before = project.next_id;
        Self {
            project,
            changes: Vec::new(),
            next_id_before,
            committed: false,
        }
    }

    /// Current state of the project, including changes made so far.
    pub fn project(&self) -> &Project {
        self.project
    }

    fn alloc(&mut self) -> u64 {
        let id = self.project.next_id;
        self.project.next_id = id.saturating_add(1);
        id
    }

    entity_ops!(
        "garment",
        GarmentId,
        Garment,
        garments,
        Garment,
        new_garment_id,
        put_garment,
        remove_garment
    );
    entity_ops!(
        "panel",
        PanelId,
        Panel,
        panels,
        Panel,
        new_panel_id,
        put_panel,
        remove_panel
    );
    entity_ops!(
        "point",
        PointId,
        Point,
        points,
        Point,
        new_point_id,
        put_point,
        remove_point
    );
    entity_ops!(
        "edge",
        EdgeId,
        Edge,
        edges,
        Edge,
        new_edge_id,
        put_edge,
        remove_edge
    );
    entity_ops!(
        "notch",
        NotchId,
        Notch,
        notches,
        Notch,
        new_notch_id,
        put_notch,
        remove_notch
    );
    entity_ops!(
        "internal line",
        InternalLineId,
        InternalLine,
        internal_lines,
        InternalLine,
        new_internal_line_id,
        put_internal_line,
        remove_internal_line
    );

    pub fn set_name(&mut self, name: String) {
        let before = mem::replace(&mut self.project.name, name.clone());
        self.changes.push(Change::Name {
            before,
            after: name,
        });
    }

    entity_ops!(
        "seam",
        SeamId,
        Seam,
        seams,
        Seam,
        new_seam_id,
        put_seam,
        remove_seam
    );

    entity_ops!(
        "pin", PinId, Pin, pins, Pin, new_pin_id, put_pin, remove_pin
    );

    entity_ops!(
        "constraint",
        ConstraintId,
        Constraint,
        constraints,
        Constraint,
        new_constraint_id,
        put_constraint,
        remove_constraint
    );

    entity_ops!(
        "guide",
        GuideId,
        Guide,
        guides,
        Guide,
        new_guide_id,
        put_guide,
        remove_guide
    );

    entity_ops!(
        "button",
        ButtonId,
        Button,
        buttons,
        Button,
        new_button_id,
        put_button,
        remove_button
    );

    entity_ops!(
        "zipper",
        ZipperId,
        Zipper,
        zippers,
        Zipper,
        new_zipper_id,
        put_zipper,
        remove_zipper
    );

    /// What changed so far, in order.
    pub fn changes(&self) -> &[Change] {
        &self.changes
    }

    pub fn set_avatar(&mut self, avatar: Option<Avatar>) {
        let before = mem::replace(&mut self.project.avatar, avatar.clone());
        self.changes.push(Change::Avatar {
            before,
            after: avatar,
        });
    }

    pub fn set_grading(&mut self, grading: Option<Grading>) {
        let before = mem::replace(&mut self.project.grading, grading.clone());
        self.changes.push(Change::Grading {
            before,
            after: grading,
        });
    }

    pub fn set_variables(&mut self, variables: BTreeMap<String, String>) {
        let before = mem::replace(&mut self.project.variables, variables.clone());
        self.changes.push(Change::Variables {
            before,
            after: variables,
        });
    }

    pub fn set_display_unit(&mut self, unit: DisplayUnit) {
        let before = mem::replace(&mut self.project.display_unit, unit);
        self.changes.push(Change::DisplayUnit {
            before,
            after: unit,
        });
    }

    /// Finish the transaction and return what it changed.
    pub fn commit(mut self) -> ChangeSet {
        self.committed = true;
        ChangeSet {
            changes: mem::take(&mut self.changes),
            next_id_before: self.next_id_before,
            next_id_after: self.project.next_id,
        }
    }
}

impl Drop for Transaction<'_> {
    fn drop(&mut self) {
        if !self.committed {
            for c in self.changes.iter().rev() {
                c.apply(self.project, false);
            }
            self.project.next_id = self.next_id_before;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::DVec2;

    fn add_point(tx: &mut Transaction<'_>, panel: PanelId, x: f64) -> PointId {
        let id = tx.new_point_id();
        tx.put_point(id, Point::new(panel, DVec2::new(x, 0.0)));
        id
    }

    #[test]
    fn undo_redo_restores_exact_state() {
        let mut p = Project::new("t");
        let start = p.clone();
        let mut tx = Transaction::begin(&mut p);
        let panel = tx.new_panel_id();
        add_point(&mut tx, panel, 1.0);
        tx.set_name("renamed".into());
        let cs = tx.commit();
        let after = p.clone();
        assert_ne!(after, start);
        cs.undo(&mut p);
        assert_eq!(p, start);
        cs.redo(&mut p);
        assert_eq!(p, after);
    }

    #[test]
    fn dropped_transaction_rolls_back() {
        let mut p = Project::new("t");
        let start = p.clone();
        {
            let mut tx = Transaction::begin(&mut p);
            let panel = tx.new_panel_id();
            add_point(&mut tx, panel, 1.0);
        }
        assert_eq!(p, start);
    }

    #[test]
    fn panels_lists_what_exists_once() {
        let mut p = Project::new("t");
        let mut tx = Transaction::begin(&mut p);
        let gone = tx.new_panel_id();
        add_point(&mut tx, gone, 0.0);
        let cs = tx.commit();
        // The panel itself was never put: only existing panels count.
        assert!(cs.panels(&p).is_empty());
    }

    #[test]
    fn append_compacts_per_entity() {
        let mut p = Project::new("t");
        let start = p.clone();
        let mut tx = Transaction::begin(&mut p);
        let panel = tx.new_panel_id();
        let pt = add_point(&mut tx, panel, 0.0);
        let mut cs = tx.commit();
        for i in 1..=100 {
            let mut tx = Transaction::begin(&mut p);
            tx.put_point(pt, Point::new(panel, DVec2::new(i as f64, 0.0)));
            cs.append(tx.commit());
        }
        assert_eq!(cs.changes.len(), 1);
        let end = p.clone();
        cs.undo(&mut p);
        assert_eq!(p, start);
        cs.redo(&mut p);
        assert_eq!(p, end);
    }
}
