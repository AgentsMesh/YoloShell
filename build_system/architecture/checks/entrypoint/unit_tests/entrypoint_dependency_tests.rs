use super::entrypoint_fixture::*;
use crate::constants as contract;

#[test]
fn only_role_entry_and_minimal_dispatch_dependencies_are_admitted() {
    for dependency in [
        BUSINESS,
        FORBIDDEN_RUNTIME,
        "//core/crates/hub/client:yolo_hub_client",
    ] {
        let report = report(ENTRY.replace(ALLOWED, dependency));
        assert!(report.contains(contract::DEPENDENCY_POLICY), "{report}");
    }
}

#[test]
fn named_dependency_lists_resolve_and_cannot_smuggle_business_dependencies() {
    let valid = ENTRY.replace("deps = [", "deps = DEPS + [");
    let admitted = report(format!("DEPS = [{ALLOWED:?}]\n{valid}"));
    assert!(admitted.is_empty(), "{admitted}");
    let rejected = report(format!("DEPS = [{BUSINESS:?}]\n{valid}"));
    assert!(rejected.contains(contract::DEPENDENCY_POLICY), "{rejected}");
}

#[test]
fn unknown_dynamic_cyclic_or_reassigned_dependencies_fail_closed() {
    let build = ENTRY.replace("deps = [", "deps = DEPS + [");
    for binding in [
        "",
        "DEPS = select({})\n",
        "DEPS = DEPS\n",
        "DEPS = []\nDEPS = []\n",
        "DEPS = UNKNOWN\n",
    ] {
        let report = report(format!("{binding}{build}"));
        assert!(report.contains(contract::DEPENDENCY_POLICY), "{report}");
    }
}

#[test]
fn comment_and_string_decoys_neither_add_binaries_nor_hide_real_dependencies() {
    let decoys = "# rust_binary(name = \"helper\")\nTEXT = 'rust_binary(name = \"helper\")'\n";
    let accepted = report(format!("{decoys}{ENTRY}"));
    assert!(accepted.is_empty(), "{accepted}");
    let rejected = report(format!("{decoys}{}", ENTRY.replace(ALLOWED, BUSINESS)));
    assert!(rejected.contains(contract::DEPENDENCY_POLICY), "{rejected}");
}

#[test]
fn launch_contract_exception_cannot_acquire_business_or_external_dependencies() {
    use super::support::{inspect, messages, source};
    for dependency in [BUSINESS, FORBIDDEN_RUNTIME, ALLOWED] {
        let mut inventory = files(ENTRY);
        inventory.push(source(
            &format!("{}/BUILD.bazel", contract::LAUNCH_PACKAGE),
            format!("rust_library(name=\"library\", srcs=[\"src/lib.rs\"], deps=[{dependency:?}])"),
        ));
        inventory.push(source(
            &format!("{}/src/lib.rs", contract::LAUNCH_PACKAGE),
            "pub struct Value;",
        ));
        let report = messages(&inspect(inventory));
        assert!(report.contains(contract::PURE_LAUNCH_POLICY), "{report}");
    }
}

mod plugins {
    use yolo_architecture_foundation::SourceFile;

    fn inspect(
        rule: &str,
        dependencies: &str,
        path: &str,
    ) -> Vec<yolo_architecture_foundation::model::Violation> {
        let text = format!(
            "load(\"@rules_rust//rust:defs.bzl\", \"{rule}\")\nSHARED = {dependencies}\n{rule}(name = \"component\", srcs = [], deps = SHARED)\n"
        );
        let files = [SourceFile {
            absolute: path.into(),
            relative: path.into(),
            text,
        }];
        let repository = crate::Repository::new(&files);
        let mut violations = Vec::new();
        crate::dependencies::plugin::inspect(&repository, &mut violations);
        violations
    }

    #[test]
    fn core_cannot_import_ui_through_dependency_variables() {
        for dependency in [
            "//plugins/tui/app:yolo_tui_app",
            "@crates//:ratatui",
            "@crates//:crossterm",
        ] {
            assert_eq!(
                inspect(
                    crate::constants::RUST_LIBRARY_RULE,
                    &format!("[\"{dependency}\"]"),
                    "core/crates/hub/BUILD.bazel"
                )
                .len(),
                1
            );
        }
    }

    #[test]
    fn composition_and_test_consumers_can_load_plugins() {
        let dependency = "[\"//plugins/tui/app:yolo_tui_app\"]";
        assert!(
            inspect(
                crate::constants::RUST_LIBRARY_RULE,
                dependency,
                "core/crates/application/yolo/BUILD.bazel"
            )
            .is_empty()
        );
        assert!(
            inspect(
                crate::constants::RUST_TEST_RULE,
                dependency,
                "core/crates/hub/BUILD.bazel"
            )
            .is_empty()
        );
        assert!(
            inspect(
                crate::constants::RUST_LIBRARY_RULE,
                dependency,
                "plugins/tui/cli/BUILD.bazel"
            )
            .is_empty()
        );
        assert!(
            inspect(
                crate::constants::RUST_LIBRARY_RULE,
                "[\"//core/crates/plugin/client/contract:yolo_builtin_client_contract\"]",
                "core/crates/hub/BUILD.bazel"
            )
            .is_empty()
        );
    }
}
