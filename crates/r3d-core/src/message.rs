//! Structured, translatable messages.
//!
//! Core crates never build user-facing prose. Errors and notices carry a
//! stable `code` plus named `args`; front-ends translate them with their
//! message catalog. The English `text` is a fallback for logs and the CLI.
//! See `docs/architecture.md` §7.7.

use std::collections::BTreeMap;

use serde::Serialize;

/// A translatable message: stable code, named arguments, English fallback.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Message {
    /// Stable identifier, e.g. `"entity_not_found"`. Never changes once
    /// published; it is the translation key.
    pub code: &'static str,
    /// Named arguments referenced by translations.
    pub args: BTreeMap<&'static str, String>,
    /// English text, for logs and as a fallback.
    pub text: String,
}

impl Message {
    pub fn new(code: &'static str, text: impl Into<String>) -> Self {
        Self {
            code,
            args: BTreeMap::new(),
            text: text.into(),
        }
    }

    #[must_use]
    pub fn arg(mut self, name: &'static str, value: impl ToString) -> Self {
        self.args.insert(name, value.to_string());
        self
    }
}

/// Anything that can be shown to a user in any language.
pub trait Localizable {
    fn message(&self) -> Message;
}
