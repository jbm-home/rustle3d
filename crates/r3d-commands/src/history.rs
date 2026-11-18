//! Undo/redo history of change sets.

use r3d_core::{ChangeSet, PanelId, Project};

/// Default number of undo steps kept.
pub const DEFAULT_LIMIT: usize = 500;

#[derive(Debug)]
struct Entry {
    label: String,
    merge_key: Option<String>,
    changes: ChangeSet,
}

/// Linear undo/redo history.
#[derive(Debug)]
pub struct History {
    undo: Vec<Entry>,
    redo: Vec<Entry>,
    limit: usize,
    sealed: bool,
}

impl Default for History {
    fn default() -> Self {
        Self::with_limit(DEFAULT_LIMIT)
    }
}

/// What undoing or redoing steps changed: the last step's label and the
/// panels touched that still exist (to select them).
#[derive(Debug, Default, PartialEq)]
pub struct Moved {
    pub label: String,
    pub panels: Vec<PanelId>,
}

impl Moved {
    fn add(&mut self, entry: &Entry, project: &Project) {
        self.label.clone_from(&entry.label);
        for id in entry.changes.panels(project) {
            if !self.panels.contains(&id) {
                self.panels.push(id);
            }
        }
    }
}

impl History {
    pub fn with_limit(limit: usize) -> Self {
        Self {
            undo: Vec::new(),
            redo: Vec::new(),
            limit: limit.max(1),
            sealed: true,
        }
    }

    /// Record a committed change set. Empty change sets are ignored. If the
    /// merge key matches the previous entry (and no seal happened in
    /// between), the two merge into one undo step.
    pub fn push(&mut self, label: &str, merge_key: Option<String>, changes: ChangeSet) {
        if changes.is_empty() {
            return;
        }
        self.redo.clear();
        if let (false, Some(key), Some(top)) = (self.sealed, &merge_key, self.undo.last_mut())
            && top.merge_key.as_ref() == Some(key)
        {
            top.changes.append(changes);
            return;
        }
        self.undo.push(Entry {
            label: label.to_owned(),
            merge_key,
            changes,
        });
        self.sealed = false;
        if self.undo.len() > self.limit {
            self.undo.remove(0);
        }
    }

    /// End the current merge run (e.g. on mouse release).
    pub fn seal(&mut self) {
        self.sealed = true;
    }

    /// Undo the last step.
    pub fn undo(&mut self, project: &mut Project) -> Option<Moved> {
        let at = self.undo.len().checked_sub(1)?;
        self.go_to(project, at)
    }

    /// Redo the last undone step.
    pub fn redo(&mut self, project: &mut Project) -> Option<Moved> {
        if self.redo.is_empty() {
            return None;
        }
        self.go_to(project, self.undo.len() + 1)
    }

    /// Undo or redo steps until `at` steps are done (0: none, all undone;
    /// [`History::len`]: all redone). `None` past the end or where it is
    /// already.
    pub fn go_to(&mut self, project: &mut Project, at: usize) -> Option<Moved> {
        if at > self.len() || at == self.undo.len() {
            return None;
        }
        let mut moved = Moved::default();
        while self.undo.len() > at {
            let entry = self.undo.pop()?;
            entry.changes.undo(project);
            moved.add(&entry, project);
            self.redo.push(entry);
        }
        while self.undo.len() < at {
            let entry = self.redo.pop()?;
            entry.changes.redo(project);
            moved.add(&entry, project);
            self.undo.push(entry);
        }
        self.sealed = true;
        Some(moved)
    }

    /// Every step's label, oldest first: the done ones, then the undone.
    pub fn labels(&self) -> Vec<&str> {
        self.undo
            .iter()
            .chain(self.redo.iter().rev())
            .map(|e| e.label.as_str())
            .collect()
    }

    /// Steps done: what undo would take back, from the latest.
    pub fn undo_len(&self) -> usize {
        self.undo.len()
    }

    pub fn redo_len(&self) -> usize {
        self.redo.len()
    }

    /// Steps done and undone.
    pub fn len(&self) -> usize {
        self.undo.len() + self.redo.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
