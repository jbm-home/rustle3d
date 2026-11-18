//! The dress forms of an engine: the one for the shown body, and a body
//! being fitted on a thread of its own (seconds), the 3D view keeping the
//! old body meanwhile.

use std::sync::Arc;

use r3d_commands::drape::Form;
use r3d_core::Body;

#[derive(Default)]
pub(crate) struct Forms {
    pub(crate) ready: Option<(Body, Arc<Form>)>,
    pub(crate) fitting: Option<Body>,
}

impl Forms {
    /// The form of `body`, if it is the one ready.
    pub(crate) fn of(&self, body: Body) -> Option<Arc<Form>> {
        self.ready
            .as_ref()
            .filter(|(b, _)| *b == body)
            .map(|(_, f)| f.clone())
    }
}
