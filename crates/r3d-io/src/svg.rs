//! SVG export at 1:1 scale (1 user unit = 1 mm).

use std::fmt::Write;

use crate::drawing::{Drawing, Stroke};

fn style(stroke: Stroke) -> String {
    match stroke {
        Stroke::Size(k) => format!(
            r##"stroke="#{:06X}" stroke-width="0.4""##,
            Stroke::size_color(k)
        ),
        other => fixed_style(other).to_owned(),
    }
}

fn fixed_style(stroke: Stroke) -> &'static str {
    match stroke {
        Stroke::Size(_) | Stroke::Cut => r##"stroke="#000" stroke-width="0.5""##,
        Stroke::Sew => r##"stroke="#000" stroke-width="0.3" stroke-dasharray="4 2""##,
        Stroke::Internal => r##"stroke="#444" stroke-width="0.3" stroke-dasharray="6 2 1 2""##,
        Stroke::Notch => r##"stroke="#000" stroke-width="0.5""##,
        Stroke::Grain => r##"stroke="#444" stroke-width="0.3""##,
        Stroke::Mark => r##"stroke="#000" stroke-width="0.3""##,
    }
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Render the drawing as a standalone SVG document. The board's y axis
/// points up; SVG's points down, so y is flipped.
pub fn to_svg(d: &Drawing) -> String {
    let size = d.bounds.max - d.bounds.min;
    let x = |v: f64| v - d.bounds.min.x;
    let y = |v: f64| d.bounds.max.y - v;
    let mut out = String::new();
    let _ = writeln!(
        out,
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{w:.3}mm" height="{h:.3}mm" viewBox="0 0 {w:.3} {h:.3}">"#,
        w = size.x,
        h = size.y
    );
    for line in &d.lines {
        let pts: Vec<String> = line
            .points
            .iter()
            .map(|p| format!("{:.3},{:.3}", x(p.x), y(p.y)))
            .collect();
        let tag = if line.closed { "polygon" } else { "polyline" };
        let _ = writeln!(
            out,
            r#"  <{tag} points="{}" fill="none" {}/>"#,
            pts.join(" "),
            style(line.stroke)
        );
    }
    for label in &d.labels {
        let _ = writeln!(
            out,
            r#"  <text x="{:.3}" y="{:.3}" font-family="Helvetica, Arial, sans-serif" font-size="{:.3}" text-anchor="middle" dominant-baseline="middle">{}</text>"#,
            x(label.pos.x),
            y(label.pos.y),
            label.size_mm,
            escape(&label.text)
        );
    }
    out.push_str("</svg>\n");
    out
}
