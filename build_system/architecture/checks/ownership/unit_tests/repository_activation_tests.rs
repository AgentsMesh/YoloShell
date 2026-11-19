use super::support::{messages, source};

fn inspect(files: &[yolo_architecture_foundation::SourceFile]) -> String {
    let mut violations = Vec::new();
    crate::inspect_repository(files, &mut violations);
    messages(&violations)
}

#[test]
fn partial_repository_inventory_does_not_run_the_global_graph() {
    let files = [source("core/crates/model/src/lib.rs", "pub struct Model;")];
    assert!(inspect(&files).is_empty());
}

#[test]
fn policy_boundary_activates_the_global_graph() {
    let files = [
        source(crate::OWNERSHIP_POLICY_BUILD_FILE, ""),
        source("core/crates/model/BUILD.bazel", ""),
        source("core/crates/model/src/lib.rs", "pub struct Model;"),
    ];
    let report = inspect(&files);
    assert!(
        report.contains(crate::constants::MISSING_OWNER_MESSAGE),
        "{report}"
    );
}
