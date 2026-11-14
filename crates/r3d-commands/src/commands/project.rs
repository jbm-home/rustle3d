use r3d_core::{DisplayUnit, Transaction};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::error::{CommandError, non_blank};
use crate::spec::{CommandOutput, CommandSpec, Described};

/// Rename the project.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProjectRename {
    /// New project name.
    pub name: String,
}

impl Described for ProjectRename {
    const SUMMARY: &'static str = "Rename the project.";
}

impl CommandSpec for ProjectRename {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        non_blank("name", &self.name)?;
        tx.set_name(self.name.trim().to_owned());
        Ok(CommandOutput::default())
    }
}

/// Choose the unit used to display lengths (storage is always mm).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProjectSetDisplayUnit {
    /// `mm`, `cm` or `in`.
    pub unit: DisplayUnit,
}

impl Described for ProjectSetDisplayUnit {
    const SUMMARY: &'static str = "Choose the unit used to display lengths (storage is always mm).";
}

impl CommandSpec for ProjectSetDisplayUnit {
    fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
        tx.set_display_unit(self.unit);
        Ok(CommandOutput::default())
    }
}
