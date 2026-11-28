use yolo_architecture_foundation::SourceFile;
use yolo_architecture_target_ownership::OWNERSHIP_POLICY_BUILD_FILE;

pub const BOUNDARIES: &str = "build_system/architecture/hub_roles.bzl";
pub const ALPHA_BUILD: &str = "core/crates/hub/alpha/BUILD.bazel";
pub const ALPHA: &str = "rust_library(name = \"lib\", srcs = [\"src/lib.rs\"])\n";

pub fn source(path: &str, text: impl Into<String>) -> SourceFile {
    SourceFile {
        absolute: path.into(),
        relative: path.to_owned(),
        text: text.into(),
    }
}

fn quoted(items: &[&str]) -> String {
    items
        .iter()
        .map(|item| format!("\"{item}\""))
        .collect::<Vec<_>>()
        .join(", ")
}

pub fn boundaries(public: &[&str], client: &[&str], entry: &[&str], runtime: &[&str]) -> String {
    format!(
        "PUBLIC_COMPONENTS = [{}]\n\
         CLIENT_COMPONENTS = [{}]\n\
         HUB_ENTRY_COMPONENTS = [{}]\n\
         RUNTIME_COMPONENTS = [{}]\n",
        quoted(public),
        quoted(client),
        quoted(entry),
        quoted(runtime),
    )
}

pub fn files(boundaries_text: &str, builds: &[(&str, &str)]) -> Vec<SourceFile> {
    let mut result = vec![
        source(OWNERSHIP_POLICY_BUILD_FILE, ""),
        source(BOUNDARIES, boundaries_text),
    ];
    result.extend(
        builds
            .iter()
            .map(|(path, text)| source(path, *text)),
    );
    result
}

pub fn messages(files: Vec<SourceFile>) -> Vec<String> {
    let mut violations = Vec::new();
    crate::inspect(&files, &mut violations);
    violations
        .into_iter()
        .map(|violation| violation.to_string())
        .collect()
}

pub fn assert_reports(report: &[String], needle: &str) {
    assert!(
        report.iter().any(|message| message.contains(needle)),
        "{report:?}"
    );
}
