use yolo_architecture_test_support::Fixture;

const GENERATED_SOURCE: &str = "core/src/generated/contract.rs";
const RESOLUTION_POLICY: &str = "retained authority resolution is indeterminate";

#[test]
fn missing_generated_include_is_resolved_from_runfiles() {
    let workspace = workspace_with_generated_include();
    let runfiles = Fixture::new();
    runfiles.write(GENERATED_SOURCE, "pub(crate) struct GeneratedContract;\n");

    assert!(messages(crate::inspect(workspace.root()).unwrap()).contains(RESOLUTION_POLICY));
    let overlaid = crate::inspect_with_runfiles(workspace.root(), Some(runfiles.root())).unwrap();
    assert!(!messages(overlaid).contains(RESOLUTION_POLICY));
}

#[test]
fn distinct_runfile_cannot_replace_a_workspace_source() {
    let workspace = workspace_with_generated_include();
    let runfiles = Fixture::new();
    workspace.write(GENERATED_SOURCE, "pub(crate) struct WorkspaceContract;\n");
    runfiles.write(GENERATED_SOURCE, "pub(crate) struct RunfilesContract;\n");

    let error = crate::inspect_with_runfiles(workspace.root(), Some(runfiles.root())).unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
    assert!(error.to_string().contains(GENERATED_SOURCE));
}

#[cfg(unix)]
#[test]
fn runfile_link_to_the_workspace_keeps_one_source_identity() {
    let workspace = workspace_with_generated_include();
    let runfiles = Fixture::new();
    workspace.write(GENERATED_SOURCE, "pub(crate) struct GeneratedContract;\n");
    let link = runfiles.root().join(GENERATED_SOURCE);
    std::fs::create_dir_all(link.parent().unwrap()).unwrap();
    std::os::unix::fs::symlink(workspace.root().join(GENERATED_SOURCE), link).unwrap();

    crate::inspect_with_runfiles(workspace.root(), Some(runfiles.root())).unwrap();
}

fn workspace_with_generated_include() -> Fixture {
    let fixture = Fixture::new();
    fixture.write(
        "core/BUILD.bazel",
        "rust_library(name = \"runtime\", crate_root = \"src/lib.rs\")\n",
    );
    fixture.write("core/src/lib.rs", "include!(\"generated/contract.rs\");\n");
    fixture
}

fn messages(violations: Vec<crate::Violation>) -> String {
    violations
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("\n")
}
