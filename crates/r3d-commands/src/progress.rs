//! Progress of long steps, and stopping them.
//!
//! A step that takes seconds (a template drafted in every size of a range,
//! a headless drape) reports how far it is through a [`Progress`] handed
//! to it, and stops with [`CommandError::Cancelled`] when asked: the
//! transaction is dropped, so nothing is changed. Front-ends choose what
//! to do with the reports: the app shows a bar, the CLI prints lines, the
//! MCP server sends progress notifications.

use std::fmt;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use serde::Serialize;

use crate::error::CommandError;

/// Where a long step is.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Status {
    /// What is being done, a stable code for translation:
    /// `template.sizes` (drafting a template in every size; `item` is the
    /// size just drafted) or `simulate.frames` (draping headless).
    pub task: &'static str,
    /// Parts done so far, of `total`.
    pub done: u32,
    /// Parts in all.
    pub total: u32,
    /// The part just done, if it has a name (a size).
    #[serde(skip_serializing_if = "String::is_empty")]
    pub item: String,
}

type Report = Arc<dyn Fn(&Status) + Send + Sync>;

/// Handed to long steps: where to report, and whether to stop. The
/// default reports nowhere and never stops.
#[derive(Clone, Default)]
pub struct Progress {
    report: Option<Report>,
    cancel: Option<Arc<AtomicBool>>,
}

impl fmt::Debug for Progress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Progress")
            .field("cancelled", &self.cancelled())
            .finish_non_exhaustive()
    }
}

impl Progress {
    /// Report to `report` (called from any thread, possibly several at
    /// once); stop when `cancel` is set.
    pub fn new(report: impl Fn(&Status) + Send + Sync + 'static, cancel: Arc<AtomicBool>) -> Self {
        Self {
            report: Some(Arc::new(report)),
            cancel: Some(cancel),
        }
    }

    pub fn report(&self, status: &Status) {
        if let Some(report) = &self.report {
            report(status);
        }
    }

    /// Whether the step was asked to stop.
    pub fn cancelled(&self) -> bool {
        self.cancel
            .as_ref()
            .is_some_and(|c| c.load(Ordering::Relaxed))
    }

    /// `Err(Cancelled)` once the step was asked to stop: call between
    /// parts of the work.
    pub fn check(&self) -> Result<(), CommandError> {
        if self.cancelled() {
            Err(CommandError::Cancelled)
        } else {
            Ok(())
        }
    }
}
