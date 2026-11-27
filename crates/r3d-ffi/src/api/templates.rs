//! What the start screen shows: the starter garments (their sizes for a
//! body) and pictures of patterns, of templates and of the user's
//! projects, drawn by our renderer.

use std::path::Path;

use flutter_rust_bridge::frb;
use r3d_commands::{Query, Session, Step, Workspace};
use r3d_core::{Body, Project};
use r3d_render::board::BoardView;
use r3d_render::offscreen::{PNG_FORMAT, render_png};
use r3d_render::{BoardRenderer, BoardScene, Target};

use super::engine::{ApiError, CanvasColors, err};

fn body_of(body_json: &str) -> Result<Body, ApiError> {
    serde_json::from_str(body_json).map_err(|e| ApiError::new("malformed", e.to_string()))
}

fn session_for(body: Body) -> Result<Session, ApiError> {
    let mut s = Session::new(Project::new("Preview"));
    let step =
        Step::from_value(serde_json::json!({ "type": "avatar.set", "body": body })).map_err(err)?;
    s.run_step(&step).map_err(err)?;
    Ok(s)
}

/// The version of Rustle3D (the workspace's).
#[frb(sync)]
pub fn app_version() -> String {
    env!("CARGO_PKG_VERSION").to_owned()
}

/// The templates and their default sizes for `body_json` (a `Body`), as
/// the `template.list` query returns them. Fitting the body takes a
/// second or more (a child's longer): asynchronous for the UI.
pub fn template_list(body_json: String) -> Result<String, ApiError> {
    let s = session_for(body_of(&body_json)?)?;
    let list = s
        .query(&Query::TemplateList(r3d_commands::queries::TemplateList {}))
        .map_err(err)?;
    Ok(list.to_string())
}

/// The built-in size charts as the `grading.charts` query gives them,
/// without a body (`nearest` null): a new project's body is chosen from
/// them (spec 37).
#[frb(sync)]
pub fn size_charts() -> Result<String, ApiError> {
    let s = Session::new(Project::new("Preview"));
    let charts = s
        .query(&Query::GradingCharts(
            r3d_commands::queries::GradingCharts {},
        ))
        .map_err(err)?;
    Ok(charts.to_string())
}

/// The body (a `Body` as JSON) of `size` on `chart`, shaped by
/// `silhouette`, `height_cm` tall when given: the `avatar.size_body`
/// query.
#[frb(sync)]
pub fn size_body(
    chart: String,
    size: String,
    silhouette: String,
    height_cm: Option<f64>,
) -> Result<String, ApiError> {
    let mut s = Session::new(Project::new("Preview"));
    let step = Step::from_value(serde_json::json!({
        "type": "avatar.size_body", "chart": chart, "size": size,
        "silhouette": silhouette, "height_cm": height_cm,
    }))
    .map_err(err)?;
    Ok(s.run_step(&step).map_err(err)?.to_string())
}

/// The chart and size nearest `body_json` (a `Body`): `{chart, name}`.
#[frb(sync)]
pub fn nearest_size(body_json: String) -> Result<String, ApiError> {
    let body = body_of(&body_json)?;
    let chart = r3d_commands::commands::SizeChart::of(&body);
    Ok(serde_json::json!({"chart": chart, "name": chart.nearest(&body)}).to_string())
}

/// A PNG picture of `template`'s pattern pieces at default sizes for a
/// standard body, on the board colours of `colors`.
pub fn template_thumbnail(
    template: String,
    width: u32,
    height: u32,
    colors: CanvasColors,
) -> Result<Vec<u8>, ApiError> {
    let mut s = Session::new(Project::new("Preview"));
    let step =
        Step::from_value(serde_json::json!({ "type": "template.apply", "template": template }))
            .map_err(err)?;
    s.run_step(&step).map_err(err)?;
    pattern_png(s.project(), width, height, &colors)
}

/// A PNG picture of the pattern of the project folder at `path` (read
/// only), on the board colours of `colors`.
pub fn project_thumbnail(
    path: String,
    width: u32,
    height: u32,
    colors: CanvasColors,
) -> Result<Vec<u8>, ApiError> {
    let ws = Workspace::open(Path::new(&path)).map_err(err)?;
    pattern_png(ws.session().project(), width, height, &colors)
}

fn pattern_png(
    project: &Project,
    width: u32,
    height: u32,
    colors: &CanvasColors,
) -> Result<Vec<u8>, ApiError> {
    let theme = colors.to_theme()?;
    let gpu = crate::surface::ui_gpu().map_err(|e| ApiError::new("gpu", e.to_string()))?;
    let scene = BoardScene::from_project(project, &[]);
    let view = BoardView::fit(scene.bounds(), width, height);
    let mut renderer = BoardRenderer::new(&gpu, PNG_FORMAT);
    render_png(&gpu, width, height, |target| {
        let target = Target {
            view: target,
            width,
            height,
        };
        renderer.render(&gpu, target, &scene, &view, &theme);
    })
    .map_err(|e| ApiError::new("gpu", e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes_follow_the_body() {
        let body = r#"{"masculinity":1,"height_cm":182,"bust_cm":104,"waist_cm":90,"hips_cm":102}"#;
        let list: serde_json::Value =
            serde_json::from_str(&template_list(body.into()).unwrap()).unwrap();
        assert_eq!(list[0]["template"], "t_shirt");
        assert_eq!(list[0]["sizes"]["chest_cm"], 104.0);
        assert!(template_list("{}".into()).is_err());
    }
}
