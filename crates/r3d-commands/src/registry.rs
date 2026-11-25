//! The list of every command and query. Adding an operation = implement its
//! spec, then add one line here. Docs, CLI and MCP pick it up automatically.

use r3d_core::{Project, Transaction};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::actions::*;
use crate::commands::*;
use crate::error::CommandError;
use crate::fit::{FitLength, FitReport, FitTape};
use crate::game::ExportGame;
use crate::production::*;
use crate::progress::Progress;
use crate::queries::*;
use crate::spec::{CommandOutput, CommandSpec, Info, Named, QuerySpec, schema_value};
use crate::techpack::ExportTechPack;
use crate::workspace::{Workspace, WorkspaceError};

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
            pub(crate) fn apply(&self, tx: &mut Transaction<'_>, progress: &Progress) -> Result<CommandOutput, CommandError> {
                match self { $( Command::$T(c) => c.apply_with(tx, progress), )* }
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

macro_rules! action_registry {
    ($($name:literal => $T:ident,)*) => {
        /// Any action, tagged by its name in the `type` field.
        #[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
        #[serde(tag = "type")]
        pub enum Action {
            $( #[serde(rename = $name)] $T($T), )*
        }

        $( impl Named for $T { const NAME: &'static str = $name; } )*

        impl Action {
            pub fn name(&self) -> &'static str {
                match self { $( Action::$T(_) => $name, )* }
            }
            pub fn run(&self, ws: &mut Workspace, progress: &Progress) -> Result<Value, WorkspaceError> {
                match self { $( Action::$T(a) => a.run_with(ws, progress), )* }
            }
            /// Every action, in registry order.
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
    "avatar.set" => AvatarSet,
    "avatar.set_measures" => AvatarSetMeasures,
    "avatar.remove" => AvatarRemove,
    "avatar.import_mesh" => AvatarImportMesh,
    "avatar.orient" => AvatarOrient,
    "avatar.set_point" => AvatarSetPoint,
    "avatar.guess_points" => AvatarGuessPoints,
    "avatar.remove_mesh" => AvatarRemoveMesh,
    "panel.create_rect" => PanelCreateRect,
    "panel.create_polygon" => PanelCreatePolygon,
    "panel.create_path" => PanelCreatePath,
    "panel.rename" => PanelRename,
    "panel.translate" => PanelTranslate,
    "panel.rotate" => PanelRotate,
    "panel.scale" => PanelScale,
    "panel.set_grain" => PanelSetGrain,
    "panel.set_seam_allowance" => PanelSetSeamAllowance,
    "panel.mirror" => PanelMirror,
    "panel.unlink" => PanelUnlink,
    "panel.place" => PanelPlace,
    "panel.delete" => PanelDelete,
    "panel.cut" => PanelCut,
    "panel.paste" => PanelPaste,
    "point.move" => PointMove,
    "point.set_distance" => PointSetDistance,
    "point.remove" => PointRemove,
    "edge.set_curve" => EdgeSetCurve,
    "edge.split" => EdgeSplit,
    "edge.move" => EdgeMove,
    "notch.add" => NotchAdd,
    "notch.move" => NotchMove,
    "notch.remove" => NotchRemove,
    "internal_line.add" => InternalLineAdd,
    "internal_line.remove" => InternalLineRemove,
    "seam.add" => SeamAdd,
    "seam.remove" => SeamRemove,
    "seam.flip" => SeamFlip,
    "seam.set_gathered" => SeamSetGathered,
    "seam.set_ease" => SeamSetEase,
    "dart.add" => DartAdd,
    "pleat.add" => PleatAdd,
    "panel.add_hole" => PanelAddHole,
    "panel.remove_hole" => PanelRemoveHole,
    "panel.boolean" => PanelBoolean,
    "edge.lengthen" => EdgeLengthen,
    "edge.set_length" => EdgeSetLength,
    "seam.take_in" => SeamTakeIn,
    "seam.match_length" => SeamMatchLength,
    "pin.add" => PinAdd,
    "pin.remove" => PinRemove,
    "button.add" => ButtonAdd,
    "button.set" => ButtonSet,
    "button.set_closed" => ButtonSetClosed,
    "button.remove" => ButtonRemove,
    "zipper.add" => ZipperAdd,
    "zipper.set_open" => ZipperSetOpen,
    "zipper.remove" => ZipperRemove,
    "elastic.add" => ElasticAdd,
    "elastic.set" => ElasticSet,
    "elastic.remove" => ElasticRemove,
    "elastic.shirr" => ElasticShirr,
    "topstitch.add" => TopstitchAdd,
    "topstitch.set" => TopstitchSet,
    "topstitch.remove" => TopstitchRemove,
    "interfacing.add" => InterfacingAdd,
    "interfacing.set" => InterfacingSet,
    "interfacing.remove" => InterfacingRemove,
    "quilting.add" => QuiltingAdd,
    "quilting.set" => QuiltingSet,
    "quilting.remove" => QuiltingRemove,
    "garment.set_fabric" => GarmentSetFabric,
    "garment.set_fabric_details" => GarmentSetFabricDetails,
    "garment.set_notes" => GarmentSetNotes,
    "garment.set_print" => GarmentSetPrint,
    "garment.remove_print" => GarmentRemovePrint,
    "panel.set_print_placement" => PanelSetPrintPlacement,
    "garment.arrange" => GarmentArrange,
    "garment.set_color" => GarmentSetColor,
    "garment.add" => GarmentAdd,
    "garment.rename" => GarmentRename,
    "garment.remove" => GarmentRemove,
    "garment.set_layer" => GarmentSetLayer,
    "garment.set_draped" => GarmentSetDraped,
    "panel.set_garment" => PanelSetGarment,
    "grading.set_sizes" => GradingSetSizes,
    "grading.use_chart" => GradingUseChart,
    "grading.clear" => GradingClear,
    "grading.show_size" => GradingShowSize,
    "point.set_grade" => PointSetGrade,
    "template.apply" => TemplateApply,
    "pattern.import" => PatternImport,
    "pattern.import_parametric" => PatternImportParametric,
    "garment.redraft" => GarmentRedraft,
    "avatar.use_measurements" => AvatarUseMeasurements,
    "constraint.add" => ConstraintAdd,
    "constraint.remove" => ConstraintRemove,
    "constraint.set_value" => ConstraintSetValue,
    "point.set_smooth" => PointSetSmooth,
    "panel.unfold" => PanelUnfold,
    "guide.add" => GuideAdd,
    "guide.move" => GuideMove,
    "guide.remove" => GuideRemove,
    "panel.create_ellipse" => PanelCreateEllipse,
    "panel.create_sector" => PanelCreateSector,
    "edge.set_arc" => EdgeSetArc,
    "constraint.set_formula" => ConstraintSetFormula,
    "constraint.rename" => ConstraintRename,
    "constraint.place_label" => ConstraintPlaceLabel,
    "variable.set" => VariableSet,
    "variable.remove" => VariableRemove,
}

query_registry! {
    "project.summary" => ProjectSummary,
    "panel.list" => PanelList,
    "panel.get" => PanelGet,
    "panel.copy" => PanelCopy,
    "measure.distance" => MeasureDistance,
    "measure.contour_length" => MeasureContourLength,
    "avatar.get" => AvatarGet,
    "avatar.presets" => AvatarPresets,
    "avatar.check" => AvatarCheck,
    "seam.list" => SeamList,
    "pin.list" => PinList,
    "trim.list" => TrimList,
    "detail.list" => DetailList,
    "template.list" => TemplateList,
    "grading.get" => GradingGet,
    "grading.check" => GradingCheck,
    "grading.charts" => GradingCharts,
    "constraint.list" => ConstraintList,
    "guide.list" => GuideList,
    "variable.list" => VariableList,
    "production.indicators" => ProductionIndicators,
    "marker.compute" => MarkerCompute,
}

action_registry! {
    "export.svg" => ExportSvg,
    "simulate" => Simulate,
    "export.pdf" => ExportPdf,
    "export.gltf" => ExportGltf,
    "export.obj" => ExportObj,
    "export.game" => ExportGame,
    "export.dxf" => ExportDxf,
    "import.pattern" => ImportPattern,
    "import.measurements" => ImportMeasurements,
    "avatar.import" => AvatarImport,
    "export.marker" => ExportMarker,
    "export.techpack" => ExportTechPack,
    "fit.report" => FitReport,
    "fit.tape" => FitTape,
    "fit.length" => FitLength,
}
