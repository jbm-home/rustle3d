//! Recorded edits: the only way to modify a [`Project`].
//!
//! A [`Transaction`] applies changes immediately and records each one as
//! `(id, before, after)`. Committing returns a [`ChangeSet`] that can be
//! undone and redone generically; dropping an uncommitted transaction rolls
//! it back, so a failing command can never leave a half-applied document.

use std::mem;

use crate::id::{EdgeId, GarmentId, InternalLineId, NotchId, PanelId, PinId, PointId, SeamId};
use crate::model::{
    Avatar, DisplayUnit, Edge, Garment, IdMap, InternalLine, Notch, Panel, Pin, Point, Project,
    Seam,
};

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
            Change::Name { before, after } => {
                p.name = if forward { after } else { before }.clone();
            }
            Change::DisplayUnit { before, after } => {
                p.display_unit = if forward { *after } else { *before };
            }
            Change::Avatar { before, after } => {
                p.avatar = if forward { after } else { before }.clone();
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
            (Change::Seam(a), Change::Seam(b)) if a.id == b.id => a.after = b.after,
            (Change::Pin(a), Change::Pin(b)) if a.id == b.id => a.after = b.after,
            (Change::InternalLine(a), Change::InternalLine(b)) if a.id == b.id => {
                a.after = b.after.clone()
            }
            (Change::Name { after, .. }, Change::Name { after: b, .. }) => *after = b.clone(),
            (Change::Avatar { after, .. }, Change::Avatar { after: b, .. }) => *after = b.clone(),
            (Change::DisplayUnit { after, .. }, Change::DisplayUnit { after: b, .. }) => {
                *after = *b
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

    pub fn set_avatar(&mut self, avatar: Option<Avatar>) {
        let before = mem::replace(&mut self.project.avatar, avatar.clone());
        self.changes.push(Change::Avatar {
            before,
            after: avatar,
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
        tx.put_point(
            id,
            Point {
                panel,
                pos: DVec2::new(x, 0.0),
            },
        );
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
    fn append_compacts_per_entity() {
        let mut p = Project::new("t");
        let start = p.clone();
        let mut tx = Transaction::begin(&mut p);
        let panel = tx.new_panel_id();
        let pt = add_point(&mut tx, panel, 0.0);
        let mut cs = tx.commit();
        for i in 1..=100 {
            let mut tx = Transaction::begin(&mut p);
            tx.put_point(
                pt,
                Point {
                    panel,
                    pos: DVec2::new(i as f64, 0.0),
                },
            );
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
