use crate::constants::{
    BAZEL_MAIN_REPOSITORY_DIRECTORY, BAZEL_RUNFILES_DIRECTORY_SUFFIX, TEST_SOURCE_DIRECTORY_ENV,
    TEST_WORKSPACE_ENV, WORKSPACE_MARKER,
};
use yolo_architecture_test_support::Fixture;

const EXECUTABLE_NAME: &str = "check";

const REQUIRED_RUNFILE_BOUNDARIES: &[&str] = &[
    "BUILD.bazel",
    "core/BUILD.bazel",
    "core/crates/agent/BUILD.bazel",
    "core/crates/agent/system/BUILD.bazel",
    "core/crates/application/BUILD.bazel",
    "core/crates/conversation/BUILD.bazel",
    "core/crates/persistence/BUILD.bazel",
    "core/crates/plugin/BUILD.bazel",
    "core/crates/presentation/BUILD.bazel",
    "core/crates/process/BUILD.bazel",
    "core/crates/runtime/engine/BUILD.bazel",
    "core/crates/terminal/BUILD.bazel",
    "core/crates/transport/BUILD.bazel",
    "core/src/generated/lifecycle_deadline_contract.rs",
    "core/src/generated/scm_rights_descriptor_contract.rs",
    "integration_tests/runtime/BUILD.bazel",
    "third_party/rules_rust/BUILD.bazel",
    "build_system/BUILD.bazel",
    "build_system/architecture/BUILD.bazel",
    "build_system/architecture/checks/cargo/BUILD.bazel",
    "build_system/architecture/foundation/BUILD.bazel",
    "build_system/architecture/contracts/BUILD.bazel",
    "build_system/architecture/checks/process/signal/foundation/BUILD.bazel",
    "build_system/architecture/graph/BUILD.bazel",
    "build_system/architecture/checks/process/signal/path/BUILD.bazel",
    "build_system/architecture/checks/process/signal/policy/BUILD.bazel",
    "build_system/architecture/checks/repository/rules/BUILD.bazel",
    "build_system/architecture/checks/repository/BUILD.bazel",
    "build_system/architecture/checks/runtime/safety/policy/BUILD.bazel",
    "build_system/architecture/checks/source/BUILD.bazel",
    "build_system/architecture/syntax/BUILD.bazel",
    "build_system/architecture/checks/ownership/BUILD.bazel",
    "build_system/architecture/test_support/BUILD.bazel",
    "build_system/platform/linux/BUILD.bazel",
    "build_system/platform/linux/amd64/BUILD.bazel",
    "build_system/platform/linux/arm64/BUILD.bazel",
    "build_system/release/BUILD.bazel",
    "build_system/release/acceptance_receipt/BUILD.bazel",
    "build_system/release/archive_audit/BUILD.bazel",
    "build_system/release/candidate_artifact/BUILD.bazel",
    "build_system/release/candidate_workflow/BUILD.bazel",
    "build_system/release/contracts/BUILD.bazel",
    "build_system/release/delivery_system/BUILD.bazel",
    "build_system/release/evidence_model/BUILD.bazel",
    "build_system/release/filesystem/BUILD.bazel",
    "build_system/release/foundation/BUILD.bazel",
    "build_system/release/gate_system/BUILD.bazel",
    "build_system/release/linux_elf/BUILD.bazel",
    "build_system/release/process_execution/BUILD.bazel",
    "build_system/release/test_support/BUILD.bazel",
    "build_system/release/tool_custody/BUILD.bazel",
    "build_system/release/verification/BUILD.bazel",
    "build_system/release/workspace_state/BUILD.bazel",
    "build_system/bazel/rust/BUILD.bazel",
    "build_system/bazel/rust/fixtures/BUILD.bazel",
    "build_system/coverage/BUILD.bazel",
    "build_system/coverage/collector/BUILD.bazel",
    "build_system/coverage/gate/BUILD.bazel",
    "build_system/coverage/protocol/BUILD.bazel",
];

#[test]
fn resolves_a_workspace_inside_a_runfiles_tree() {
    let fixture = Fixture::new();
    let workspace = "sample_workspace";
    let root = fixture.root().join(workspace);
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join(WORKSPACE_MARKER), "module(name = \"sample\")\n").unwrap();
    let located = crate::workspace::locate_runfiles(fixture.root(), Some(workspace)).unwrap();
    assert_eq!(located, root.canonicalize().unwrap());
}

#[test]
fn absent_workspace_uses_only_the_main_runfiles_repository() {
    let fixture = Fixture::new();
    let root = fixture.root().join(BAZEL_MAIN_REPOSITORY_DIRECTORY);
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join(WORKSPACE_MARKER), "module(name = \"sample\")\n").unwrap();
    let located = crate::workspace::locate_runfiles(fixture.root(), None).unwrap();
    assert_eq!(located, root.canonicalize().unwrap());
    assert!(crate::workspace::locate_runfiles(fixture.root(), Some("missing")).is_err());
}

#[test]
fn absent_environment_uses_only_the_executable_main_runfiles_repository() {
    let fixture = Fixture::new();
    let executable = fixture.root().join("bin").join(EXECUTABLE_NAME);
    let runfiles = executable.with_file_name(format!(
        "{EXECUTABLE_NAME}{BAZEL_RUNFILES_DIRECTORY_SUFFIX}"
    ));
    let root = runfiles.join(BAZEL_MAIN_REPOSITORY_DIRECTORY);
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join(WORKSPACE_MARKER), "module(name = \"sample\")\n").unwrap();
    let located = crate::workspace::locate_executable_runfiles(&executable, None).unwrap();
    assert_eq!(located, root.canonicalize().unwrap());
    assert!(crate::workspace::locate_executable_runfiles(&executable, Some("missing")).is_err());
}

#[test]
fn resolves_a_workspace_from_a_runfiles_manifest() {
    let fixture = Fixture::new();
    let root = fixture.root().join("actual");
    std::fs::create_dir_all(&root).unwrap();
    let marker = root.join(WORKSPACE_MARKER);
    std::fs::write(&marker, "module(name = \"sample\")\n").unwrap();
    let manifest = fixture.root().join("MANIFEST");
    std::fs::write(
        &manifest,
        format!("sample/{WORKSPACE_MARKER} {}\n", marker.display()),
    )
    .unwrap();
    let located = crate::workspace::locate_manifest(&manifest, Some("sample")).unwrap();
    assert_eq!(located, root.canonicalize().unwrap());
}

#[test]
fn absent_workspace_uses_only_the_main_manifest_repository() {
    let fixture = Fixture::new();
    let root = fixture.root().join("actual");
    std::fs::create_dir_all(&root).unwrap();
    let marker = root.join(WORKSPACE_MARKER);
    std::fs::write(&marker, "module(name = \"sample\")\n").unwrap();
    let manifest = fixture.root().join("MANIFEST");
    std::fs::write(
        &manifest,
        format!(
            "{BAZEL_MAIN_REPOSITORY_DIRECTORY}/{WORKSPACE_MARKER} {}\n",
            marker.display()
        ),
    )
    .unwrap();
    let located = crate::workspace::locate_manifest(&manifest, None).unwrap();
    assert_eq!(located, root.canonicalize().unwrap());
    assert!(crate::workspace::locate_manifest(&manifest, Some("missing")).is_err());
}

#[test]
fn configured_runfiles_include_every_repository_package_boundary() {
    let Some(source) = std::env::var_os(TEST_SOURCE_DIRECTORY_ENV) else {
        return;
    };
    let Some(workspace) = std::env::var_os(TEST_WORKSPACE_ENV) else {
        return;
    };
    let root = std::path::Path::new(&source).join(workspace);
    for relative in REQUIRED_RUNFILE_BOUNDARIES {
        assert!(
            root.join(relative).is_file(),
            "architecture runfiles omit {relative}"
        );
    }
}
