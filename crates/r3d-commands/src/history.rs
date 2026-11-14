//! Undo/redo history of change sets.

use r3d_core::{ChangeSet, Project};

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
        if let (false, Some(key), Some(top)) = (self.sealed, &merge_key, self.undo.last_mut()) {
            if top.merge_key.as_ref() == Some(key) {
                top.changes.append(changes);
                return;
            }
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

    /// Undo the last step; returns its label.
    pub fn undo(&mut self, project: &mut Project) -> Option<String> {
        let entry = self.undo.pop()?;
        entry.changes.undo(project);
        let label = entry.label.clone();
        self.redo.push(entry);
        self.sealed = true;
        Some(label)
    }

    /// Redo the last undone step; returns its label.
    pub fn redo(&mut self, project: &mut Project) -> Option<String> {
        let entry = self.redo.pop()?;
        entry.changes.redo(project);
        let label = entry.label.clone();
        self.undo.push(entry);
        self.sealed = true;
        Some(label)
    }

    pub fn undo_len(&self) -> usize {
        self.undo.len()
    }

    pub fn redo_len(&self) -> usize {
        self.redo.len()
    }
}
