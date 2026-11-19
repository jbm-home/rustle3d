//! The dress forms of an engine: the one for the shown body, and a body
//! being fitted on a thread of its own (seconds), the 3D view keeping the
//! old body meanwhile.

use std::sync::Arc;

use r3d_commands::CommandError;
use r3d_commands::drape::Form;
use r3d_core::{AvatarMesh, Body, Project};

/// What a dress form is made from: the built-in body fitted to
/// measurements, or a body imported from a file (spec 27).
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum FormKey {
    Fitted(Body),
    Imported(Box<AvatarMesh>),
}

impl FormKey {
    /// The form `project` drapes on (`None` without an avatar).
    pub(crate) fn of(project: &Project) -> Option<Self> {
        match project.avatar().and_then(|a| a.mesh.clone()) {
            Some(mesh) => Some(Self::Imported(Box::new(mesh))),
            None => project.shown_body().map(Self::Fitted),
        }
    }

    /// Build it (seconds for a new fitted body; an imported one is kept
    /// once built).
    pub(crate) fn build(&self) -> Result<Arc<Form>, CommandError> {
        match self {
            Self::Fitted(body) => Form::of_body(body).map(Arc::new),
            Self::Imported(mesh) => r3d_commands::bodies::form(mesh),
        }
    }

    /// A number that changes with the key (the 3D view's avatar mesh is
    /// kept by it).
    pub(crate) fn id(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let text = match self {
            Self::Fitted(b) => format!("{b:?}"),
            Self::Imported(m) => format!("{m:?}"),
        };
        let mut h = std::collections::hash_map::DefaultHasher::new();
        text.hash(&mut h);
        h.finish()
    }
}

#[derive(Default)]
pub(crate) struct Forms {
    pub(crate) ready: Option<(FormKey, Arc<Form>)>,
    pub(crate) fitting: Option<FormKey>,
}

impl Forms {
    /// The form of `key`, if it is the one ready.
    pub(crate) fn of(&self, key: &FormKey) -> Option<Arc<Form>> {
        self.ready
            .as_ref()
            .filter(|(k, _)| k == key)
            .map(|(_, f)| f.clone())
    }
}
