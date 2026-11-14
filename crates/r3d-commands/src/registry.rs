//! The list of every command and query. Adding an operation = implement its
//! spec, then add one line here. Docs, CLI and MCP pick it up automatically.

use r3d_core::{Project, Transaction};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::commands::*;
use crate::error::CommandError;
use crate::queries::*;
use crate::spec::{CommandOutput, CommandSpec, Info, Named, QuerySpec, schema_value};

macro_rules! command_registry {
    ($($name:literal => $T:ident,)*) => {
        /// Any command, tagged by its name in the `type` field.
        #[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
        #[serde(tag = "type")]
        pub enum Command {
            $( #[serde(rename = $name)] $T($T), )*
        }

        $( impl Named for $T { const NAME: &'static str = $name; } )*

        impl Command {
            pub fn name(&self) -> &'static str {
                match self { $( Command::$T(_) => $name, )* }
            }
            pub fn destructive(&self) -> bool {
                match self { $( Command::$T(_) => <$T as CommandSpec>::DESTRUCTIVE, )* }
            }
            pub fn confirmed(&self) -> bool {
                match self { $( Command::$T(c) => c.confirmed(), )* }
            }
            pub fn merge_key(&self) -> Option<String> {
                match self { $( Command::$T(c) => c.merge_key(), )* }
            }
            pub(crate) fn apply(&self, tx: &mut Transaction<'_>) -> Result<CommandOutput, CommandError> {
                match self { $( Command::$T(c) => c.apply(tx), )* }
            }
            /// Every command, in registry order.
            pub fn registry() -> Vec<Info> {
                vec![ $( Info {
                    name: $name,
                    summary: <$T as crate::spec::Described>::SUMMARY,
                    destructive: <$T as CommandSpec>::DESTRUCTIVE,
                    schema: schema_value::<$T>,
                }, )* ]
            }
        }
    };
}

macro_rules! query_registry {
    ($($name:literal => $T:ident,)*) => {
        /// Any query, tagged by its name in the `type` field.
        #[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
        #[serde(tag = "type")]
        pub enum Query {
            $( #[serde(rename = $name)] $T($T), )*
        }

        $( impl Named for $T { const NAME: &'static str = $name; } )*

        impl Query {
            pub fn name(&self) -> &'static str {
                match self { $( Query::$T(_) => $name, )* }
            }
            pub fn run(&self, project: &Project) -> Result<Value, CommandError> {
                match self { $( Query::$T(q) => q.run(project), )* }
            }
            /// Every query, in registry order.
            pub fn registry() -> Vec<Info> {
                vec![ $( Info {
                    name: $name,
                    summary: <$T as crate::spec::Described>::SUMMARY,
                    destructive: false,
                    schema: schema_value::<$T>,
                }, )* ]
            }
        }
    };
}

command_registry! {
    "project.rename" => ProjectRename,
    "project.set_display_unit" => ProjectSetDisplayUnit,
    "panel.create_rect" => PanelCreateRect,
    "panel.create_polygon" => PanelCreatePolygon,
    "panel.rename" => PanelRename,
    "panel.translate" => PanelTranslate,
    "panel.delete" => PanelDelete,
    "point.move" => PointMove,
    "edge.set_curve" => EdgeSetCurve,
}

query_registry! {
    "project.summary" => ProjectSummary,
    "panel.list" => PanelList,
    "panel.get" => PanelGet,
    "measure.distance" => MeasureDistance,
}
