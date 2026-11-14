//! `docs/commands.md` must match the registry.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // test code

#[test]
fn commands_doc_is_up_to_date() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/commands.md");
    let on_disk = std::fs::read_to_string(path).unwrap_or_default();
    let generated = r3d_commands::docs::commands_markdown();
    assert!(
        on_disk.trim_end() == generated.trim_end(),
        "docs/commands.md is out of date: run `cargo xtask gen-docs`"
    );
}
