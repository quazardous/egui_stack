//! The README's quick start asks for the crate's current version.

#[test]
fn the_readme_quick_start_names_this_version() {
    let readme = include_str!("../README.md");
    let version = env!("CARGO_PKG_VERSION");
    let wanted: String = version.split('.').take(2).collect::<Vec<_>>().join(".");
    let line = format!("egui_stack = \"{wanted}\"");
    assert!(
        readme.lines().any(|l| l.trim() == line),
        "README.md's quick start should say `{line}`; run scripts/readme-version.sh"
    );
}
