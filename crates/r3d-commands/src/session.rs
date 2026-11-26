//! A project being edited: the document plus its history.

use r3d_core::{Project, Transaction};
use serde_json::{Value, json};

use crate::error::{CommandError, Invalid};
use crate::history::{History, Moved};
use crate::progress::Progress;
use crate::registry::{Command, Query};
use crate::spec::{CommandOutput, to_value};
use crate::step::Step;

/// How a list of commands runs as one undo step.
struct Run<'a> {
    label: &'a str,
    merge_key: Option<String>,
    /// Errors say which command failed (a batch).
    indexed: bool,
    progress: &'a Progress,
}

/// A project plus its undo history. All front-ends edit through this.
#[derive(Debug)]
pub struct Session {
    project: Project,
    history: History,
    dirty: bool,
    revision: u64,
}

impl Session {
    pub fn new(project: Project) -> Self {
        Self {
            project,
            history: History::default(),
            dirty: false,
            revision: 0,
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

    /// A number that changes with every change to the document (a
    /// command, an undo, a redo): what was derived from it at one
    /// revision holds until the next.
    pub fn revision(&self) -> u64 {
        self.revision
    }

    fn changed(&mut self) {
        self.dirty = true;
        self.revision += 1;
    }

    /// Execute one command as one undo step.
    pub fn execute(&mut self, cmd: &Command) -> Result<CommandOutput, CommandError> {
        self.execute_with(cmd, &Progress::default())
    }

    /// [`Session::execute`], reporting to `progress` and stopping when it
    /// asks (long commands only; see [`Progress`]).
    pub fn execute_with(
        &mut self,
        cmd: &Command,
        progress: &Progress,
    ) -> Result<CommandOutput, CommandError> {
        let step = Run {
            label: cmd.name(),
            merge_key: cmd.merge_key(),
            indexed: false,
            progress,
        };
        self.execute_all(std::slice::from_ref(cmd), step)
            .map(|mut outs| outs.pop().unwrap_or_default())
    }

    /// Execute commands atomically as a single undo step named `label`. If
    /// any command fails, nothing is changed and the error says which one.
    fn execute_batch_with(
        &mut self,
        label: &str,
        cmds: &[Command],
        progress: &Progress,
    ) -> Result<Vec<CommandOutput>, CommandError> {
        let step = Run {
            label,
            merge_key: None,
            indexed: true,
            progress,
        };
        self.execute_all(cmds, step)
    }

    fn execute_all(
        &mut self,
        cmds: &[Command],
        step: Run<'_>,
    ) -> Result<Vec<CommandOutput>, CommandError> {
        let Run {
            label,
            merge_key,
            indexed,
            progress,
        } = step;
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
            outputs.push(cmd.apply(&mut tx, progress).map_err(|e| wrap(i, e))?);
        }
        // Constraints hold after every step; then the derived geometry
        // follows (symmetric halves, then linked mirror copies, which may
        // copy a symmetric piece).
        crate::commands::settle_constraints(&mut tx)?;
        crate::commands::settle_trims(&mut tx);
        crate::commands::settle_details(&mut tx);
        crate::commands::sync_symmetry(&mut tx)?;
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
            self.changed();
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
        self.changed();
        Ok(moved)
    }

    /// Redo the last undone step.
    pub fn redo(&mut self) -> Result<Moved, CommandError> {
        let moved = self
            .history
            .redo(&mut self.project)
            .ok_or(CommandError::NothingToRedo)?;
        self.changed();
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
            self.changed();
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
        self.run_step_with(step, &Progress::default())
    }

    /// [`Session::run_step`], reporting to `progress` and stopping when it
    /// asks (long steps only; see [`Progress`]).
    pub fn run_step_with(
        &mut self,
        step: &Step,
        progress: &Progress,
    ) -> Result<Value, CommandError> {
        match step {
            Step::Command(c) => to_value(&self.execute_with(c, progress)?),
            Step::Batch { label, commands } => {
                to_value(&self.execute_batch_with(label, commands, progress)?)
            }
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
