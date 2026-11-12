use yolo_architecture_foundation::SourceFile;
use yolo_architecture_target_entrypoint::COMPOSITION_PACKAGE;

const BASELINE: &str = "build_system/architecture/dependency_usage_baseline.bzl";
const DEPENDENCY: &str = "//core/crates/other:yolo_other";
const OPERATOR: &str = "core/crates/model/BUILD.bazel";
const BUILD: &str = r#"
_DEPS = ["//core/crates/other:yolo_other"]
rust_library(name = "model", srcs = glob(["src/*.rs"]), deps = _DEPS)
"#;

fn source(path: &str, text: impl Into<String>) -> SourceFile {
    SourceFile {
        absolute: path.into(),
        relative: path.to_owned(),
        text: text.into(),
    }
}

fn library(build: &str, body: &str) -> Vec<SourceFile> {
    vec![
        source(
            yolo_architecture_target_ownership::OWNERSHIP_POLICY_BUILD_FILE,
            "",
        ),
        source(OPERATOR, build),
        source("core/crates/model/src/lib.rs", body),
    ]
}

fn messages(files: Vec<SourceFile>) -> String {
    let mut violations = Vec::new();
    super::inspect(&files, &mut violations);
    violations
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn rejects_unused_aggregate_dependency() {
    let report = messages(library(BUILD, "pub fn placeholder() {}\n"));
    assert!(report.contains(super::POLICY_MESSAGE), "{report}");
    assert!(report.contains(DEPENDENCY), "{report}");
}

#[test]
fn admits_referenced_dependency() {
    let report = messages(library(
        BUILD,
        "pub fn placeholder() -> yolo_other::Value {}\n",
    ));
    assert!(!report.contains(super::POLICY_MESSAGE), "{report}");
}

#[test]
fn admits_baselined_dependency_and_rejects_stale_entry() {
    let mut files = library(BUILD, "pub fn placeholder() {}\n");
    files.push(source(
        BASELINE,
        format!("DEPENDENCY_USAGE_BASELINE = [\n    (\"{OPERATOR}\", \"model\", \"{DEPENDENCY}\"),\n]\n"),
    ));
    let report = messages(files);
    assert!(!report.contains(super::POLICY_MESSAGE), "{report}");

    let mut stale = library(BUILD, "pub fn placeholder() {}\n");
    stale.push(source(
        BASELINE,
        format!(
            "DEPENDENCY_USAGE_BASELINE = [\n    (\"{OPERATOR}\", \"model\", \"//core/crates/removed:removed\"),\n]\n"
        ),
    ));
    let report = messages(stale);
    assert!(report.contains(super::STALE_POLICY), "{report}");
}

#[test]
fn skips_composition_root_and_literal_lists() {
    let composition = vec![
        source(
            yolo_architecture_target_ownership::OWNERSHIP_POLICY_BUILD_FILE,
            "",
        ),
        source(&format!("{COMPOSITION_PACKAGE}/BUILD.bazel"), BUILD),
        source(
            &format!("{COMPOSITION_PACKAGE}/src/main.rs"),
            "fn main() {}\n",
        ),
        source("core/BUILD.bazel", BUILD),
        source("core/src/lib.rs", "fn main() {}\n"),
    ];
    let literal = BUILD.replace(
        "deps = _DEPS",
        "deps = [\"//core/crates/other:yolo_other\"]",
    );
    for files in [composition, library(&literal, "pub fn placeholder() {}\n")] {
        let report = messages(files);
        assert!(!report.contains(super::POLICY_MESSAGE), "{report}");
    }
}

#[test]
fn resolves_a_loaded_dependency_list() {
    let build = "load(\"//build_system:deps.bzl\", \"SHARED_DEPS\")\n\
                 _DEPS = SHARED_DEPS\n\
                 rust_library(name = \"model\", srcs = glob([\"src/*.rs\"]), deps = _DEPS)\n";
    let mut files = library(build, "pub fn placeholder() {}\n");
    files.push(source(
        "build_system/deps.bzl",
        "OTHER = \"//core/crates/other:yolo_other\"\nSHARED_DEPS = [OTHER]\n",
    ));

    let report = messages(files);
    assert!(report.contains(super::POLICY_MESSAGE), "{report}");
    assert!(report.contains(DEPENDENCY), "{report}");
}

#[test]
fn resolves_loaded_lists_composed_with_local_lists() {
    let build = "load(\"//build_system:deps.bzl\", \"SHARED\")\n\
                 _LOCAL = [\"//core/crates/local:yolo_local\"]\n\
                 rust_library(name = \"model\", srcs = glob([\"src/*.rs\"]), deps = _LOCAL + SHARED)\n";
    let mut files = library(build, "pub fn placeholder() {}\n");
    files.push(source(
        "build_system/deps.bzl",
        "SHARED = [\"//core/crates/other:yolo_other\"]\n",
    ));

    let report = messages(files);
    assert!(report.contains("//core/crates/local:yolo_local"), "{report}");
    assert!(report.contains(DEPENDENCY), "{report}");
}

#[test]
fn keeps_baseline_for_an_unresolved_dependency_list() {
    let build = "load(\"//build_system:missing.bzl\", \"MISSING\")\n\
                 rust_library(name = \"model\", srcs = glob([\"src/*.rs\"]), deps = MISSING)\n";
    let mut files = library(build, "pub fn placeholder() {}\n");
    files.push(source(
        BASELINE,
        format!("DEPENDENCY_USAGE_BASELINE = [\n    (\"{OPERATOR}\", \"model\", \"{DEPENDENCY}\"),\n]\n"),
    ));

    let report = messages(files);
    assert!(!report.contains(super::STALE_POLICY), "{report}");
    assert!(!report.contains(super::POLICY_MESSAGE), "{report}");
}

#[test]
fn skips_a_macro_generated_rust_rule() {
    let build = "load(\"//build_system/bazel/capnp:defs.bzl\", \"capnp_rust_library\")\n\
                 _DEPS = [\"//core/crates/other:yolo_other\"]\n\
                 capnp_rust_library(name = \"schema\", srcs = [\"schema.capnp\"], deps = _DEPS)\n";
    let files = library(build, "pub fn placeholder() {}\n");

    let report = messages(files);
    assert!(!report.contains(super::POLICY_MESSAGE), "{report}");
}

#[test]
fn resolves_topology_constants_through_two_loads() {
    let build = "load(\"//build_system:deps.bzl\", \"SHARED_DEPS\")\n\
                 _DEPS = SHARED_DEPS\n\
                 rust_library(name = \"model\", srcs = glob([\"src/*.rs\"]), deps = _DEPS)\n";
    let mut files = library(build, "pub fn placeholder() {}\n");
    files.push(source(
        "build_system/deps.bzl",
        "load(\"//build_system/architecture:topology.bzl\", \"YOLO_OTHER\")\nSHARED_DEPS = [YOLO_OTHER]\n",
    ));
    files.push(source(
        "build_system/architecture/topology.bzl",
        "YOLO_OTHER = \"//core/crates/other:yolo_other\"\n",
    ));

    let report = messages(files);
    assert!(report.contains(super::POLICY_MESSAGE), "{report}");
    assert!(report.contains(DEPENDENCY), "{report}");
}
