//! A project being edited: the document plus its history.

use r3d_core::{Project, Transaction};
use serde_json::{Value, json};

use crate::error::{CommandError, Invalid};
use crate::history::{History, Moved};
use crate::registry::{Command, Query};
use crate::spec::{CommandOutput, to_value};
use crate::step::Step;

/// A project plus its undo history. All front-ends edit through this.
#[derive(Debug)]
pub struct Session {
    project: Project,
    history: History,
    dirty: bool,
}

impl Session {
    pub fn new(project: Project) -> Self {
        Self {
            project,
            history: History::default(),
            dirty: false,
        }
    }

    pub fn project(&self) -> &Project {
        &self.project
    }

    pub fn history(&self) -> &History {
        &self.history
    }

    /// End the current merge run of the history.
    pub fn seal_history(&mut self) {
        self.history.seal();
    }

    /// Unsaved changes since the last [`Session::mark_saved`].
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn mark_saved(&mut self) {
        self.dirty = false;
    }

    /// Execute one command as one undo step.
    pub fn execute(&mut self, cmd: &Command) -> Result<CommandOutput, CommandError> {
        self.execute_all(
            std::slice::from_ref(cmd),
            cmd.name(),
            cmd.merge_key(),
            false,
        )
        .map(|mut outs| outs.pop().unwrap_or_default())
    }

    /// Execute commands atomically as a single undo step named `label`. If
    /// any command fails, nothing is changed and the error says which one.
    pub fn execute_batch(
        &mut self,
        label: &str,
        cmds: &[Command],
    ) -> Result<Vec<CommandOutput>, CommandError> {
        self.execute_all(cmds, label, None, true)
    }

    fn execute_all(
        &mut self,
        cmds: &[Command],
        label: &str,
        merge_key: Option<String>,
        indexed: bool,
    ) -> Result<Vec<CommandOutput>, CommandError> {
        let wrap = |i: usize, e: CommandError| if indexed { e.at(i) } else { e };
        for (i, cmd) in cmds.iter().enumerate() {
            if cmd.destructive() && !cmd.confirmed() {
                return Err(wrap(
                    i,
                    CommandError::ConfirmationRequired {
                        command: cmd.name(),
                    },
                ));
            }
        }
        let mut tx = Transaction::begin(&mut self.project);
        let mut outputs = Vec::with_capacity(cmds.len());
        for (i, cmd) in cmds.iter().enumerate() {
            outputs.push(cmd.apply(&mut tx).map_err(|e| wrap(i, e))?);
        }
        // Linked mirror copies are derived data: refresh them once per step.
        crate::commands::sync_mirrors(&mut tx)?;
        // Never commit a document that breaks an invariant: that would be a
        // bug in a command, and the transaction is rolled back on return.
        tx.project()
            .check_integrity()
            .map_err(|e| CommandError::Internal {
                detail: e.to_string(),
            })?;
        let changes = tx.commit();
        if !changes.is_empty() {
            self.dirty = true;
        }
        self.history.push(label, merge_key, changes);
        Ok(outputs)
    }

    /// Undo the last step.
    pub fn undo(&mut self) -> Result<Moved, CommandError> {
        let moved = self
            .history
            .undo(&mut self.project)
            .ok_or(CommandError::NothingToUndo)?;
        self.dirty = true;
        Ok(moved)
    }

    /// Redo the last undone step.
    pub fn redo(&mut self) -> Result<Moved, CommandError> {
        let moved = self
            .history
            .redo(&mut self.project)
            .ok_or(CommandError::NothingToRedo)?;
        self.dirty = true;
        Ok(moved)
    }

    /// Undo or redo until `at` steps of the history are done.
    pub fn go_to(&mut self, at: usize) -> Result<Moved, CommandError> {
        let len = self.history.len();
        if at > len {
            return Err(CommandError::invalid("at", Invalid::Range(0.0, len as f64)));
        }
        let moved = self
            .history
            .go_to(&mut self.project, at)
            .unwrap_or_default();
        if !moved.label.is_empty() {
            self.dirty = true;
        }
        Ok(moved)
    }

    /// The steps of the history, oldest first, and how many are done.
    pub fn history_list(&self) -> Value {
        let steps: Vec<Value> = self
            .history
            .labels()
            .into_iter()
            .map(|label| json!({ "label": label }))
            .collect();
        json!({ "steps": steps, "at": self.history.undo_len() })
    }

    pub fn query(&self, q: &Query) -> Result<Value, CommandError> {
        q.run(&self.project)
    }

    /// Run any script step and return its JSON result.
    pub fn run_step(&mut self, step: &Step) -> Result<Value, CommandError> {
        match step {
            Step::Command(c) => to_value(&self.execute(c)?),
            Step::Batch { label, commands } => to_value(&self.execute_batch(label, commands)?),
            Step::Query(q) => self.query(q),
            Step::Undo => {
                let m = self.undo()?;
                Ok(json!({ "undone": m.label, "panels": m.panels }))
            }
            Step::Redo => {
                let m = self.redo()?;
                Ok(json!({ "redone": m.label, "panels": m.panels }))
            }
            Step::GoTo(at) => {
                let m = self.go_to(*at)?;
                Ok(json!({ "at": self.history.undo_len(), "panels": m.panels }))
            }
            Step::List => Ok(self.history_list()),
            Step::Action(a) => Err(CommandError::NeedsProject { name: a.name() }),
        }
    }
}
