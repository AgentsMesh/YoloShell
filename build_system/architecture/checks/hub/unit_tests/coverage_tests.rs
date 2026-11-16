use super::support::{ALPHA, ALPHA_BUILD, assert_reports, boundaries, files, messages};

#[test]
fn admits_a_classified_production_target() {
    let report = messages(files(
        &boundaries(&["//core/crates/hub/alpha:lib"], &[], &[], &[]),
        &[(ALPHA_BUILD, ALPHA)],
    ));

    assert!(report.is_empty(), "{report:?}");
}

#[test]
fn rejects_an_unclassified_production_target() {
    let report = messages(files(
        &boundaries(&["//core/crates/hub/alpha:lib"], &[], &[], &[]),
        &[
            (ALPHA_BUILD, ALPHA),
            (
                "core/crates/hub/beta/BUILD.bazel",
                "rust_library(name = \"lib\", srcs = [\"src/lib.rs\"])\n",
            ),
        ],
    ));

    assert_eq!(report.len(), 1, "{report:?}");
    assert_reports(&report, "//core/crates/hub/beta:lib");
}

#[test]
fn rejects_a_label_without_a_declared_target() {
    let report = messages(files(
        &boundaries(&["//core/crates/hub/alpha:missing"], &[], &[], &[]),
        &[(ALPHA_BUILD, ALPHA)],
    ));

    assert_reports(&report, "no declared target");
}

#[test]
fn rejects_a_stale_classified_package() {
    let report = messages(files(
        &boundaries(&["//core/crates/hub/gamma:lib"], &[], &[], &[]),
        &[],
    ));

    assert_reports(&report, "no BUILD file");
}

#[test]
fn rejects_a_classified_package_without_a_production_target() {
    let report = messages(files(
        &boundaries(&[], &[], &[], &["//core/crates/hub/alpha:lib"]),
        &[(
            ALPHA_BUILD,
            "filegroup(name = \"lib\", srcs = [])\n\
             rust_library(name = \"support\", testonly = True, srcs = [\"test_support/lib.rs\"])\n",
        )],
    ));

    assert_reports(&report, "no production target");
}

#[test]
fn ignores_test_only_and_commented_targets() {
    let text = "PUBLIC_COMPONENTS = [\n    \"//core/crates/hub/alpha:lib\",\n    # \"//core/crates/hub/commented:lib\",\n]\nCLIENT_COMPONENTS = []\nHUB_ENTRY_COMPONENTS = []\nRUNTIME_COMPONENTS = []\n";
    let report = messages(files(
        text,
        &[(
            ALPHA_BUILD,
            "rust_library(name = \"lib\", srcs = [\"src/lib.rs\"])\n\
             rust_library(name = \"test_support\", testonly = True, srcs = [\"test_support/lib.rs\"])\n",
        )],
    ));

    assert!(report.is_empty(), "{report:?}");
}

#[test]
fn requires_a_role_for_a_generated_rust_rule() {
    let report = messages(files(
        &boundaries(&["//core/crates/hub/alpha:lib"], &[], &[], &[]),
        &[
            (ALPHA_BUILD, ALPHA),
            (
                "core/crates/hub/schema/BUILD.bazel",
                "load(\"//build_system/bazel/capnp:defs.bzl\", \"capnp_rust_library\")\n\
                 capnp_rust_library(name = \"schema\", srcs = [\"schema.capnp\"])\n",
            ),
        ],
    ));

    assert_reports(&report, "//core/crates/hub/schema:schema");
}

#[test]
fn resolves_a_topology_constant_in_a_role_list() {
    let text = "PUBLIC_COMPONENTS = [YOLO_HUB_ALPHA]\nCLIENT_COMPONENTS = []\nHUB_ENTRY_COMPONENTS = []\nRUNTIME_COMPONENTS = []\n";
    let mut inventory = files(
        text,
        &[(ALPHA_BUILD, ALPHA)],
    );
    inventory.push(super::support::source(
        "build_system/architecture/topology.bzl",
        "YOLO_HUB_ALPHA = \"//core/crates/hub/alpha:lib\"\n",
    ));

    assert!(messages(inventory).is_empty());
}
