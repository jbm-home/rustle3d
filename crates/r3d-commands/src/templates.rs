//! Built-in starting projects: scripts (as in `r3d run`) shipped with the
//! app, the CLI and the demos. The sources live in `templates/`.

/// `(name, script)` of every template.
pub const TEMPLATES: &[(&str, &str)] = &[(
    "a-line-skirt",
    include_str!("../../../templates/a-line-skirt.json"),
)];

/// The script of the template called `name`.
pub fn get(name: &str) -> Option<&'static str> {
    TEMPLATES.iter().find(|(n, _)| *n == name).map(|(_, s)| *s)
}
