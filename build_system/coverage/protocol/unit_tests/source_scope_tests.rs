use yolo_coverage_protocol::{ProductionScope, production_key, production_scope};

#[test]
fn source_scope_uses_exact_normalized_repository_roots() {
    for (source, scope) in [
        ("core/src/lib.rs", ProductionScope::Core),
        ("core/crates/wire/src/lib.rs", ProductionScope::Core),
        ("clients/tui/app/src/lib.rs", ProductionScope::Core),
        ("core/crates/transport/remote/src/lib.rs", ProductionScope::Core),
        ("build_system/release/src/main.rs", ProductionScope::Tooling),
        (
            "build_system/tool/src/nested/parser.rs",
            ProductionScope::Tooling,
        ),
    ] {
        assert_eq!(production_scope(source).unwrap(), Some(scope));
        assert_eq!(production_key(source).unwrap().as_deref(), Some(source));
        assert_eq!(
            production_scope(&format!("./{source}")).unwrap(),
            Some(scope)
        );
    }
}

#[test]
fn tooling_paths_require_rust_source_directory_without_matching_lookalikes() {
    for source in [
        "build_system/tool/lib.rs",
        "build_system/tool/src_extra/lib.rs",
        "build_system/tool/src/BUILD.bazel",
        "build_system_extra/tool/src/lib.rs",
        "external/dependency/build_system/tool/src/lib.rs",
        "/workspace/build_system/tool/src/lib.rs",
    ] {
        assert_eq!(production_key(source).unwrap(), None, "{source}");
        assert_eq!(production_scope(source).unwrap(), None, "{source}");
    }
}

#[test]
fn client_scope_excludes_non_rust_ui_sources_fixtures_and_lookalikes() {
    for source in [
        "clients/tui/app/src/ui.ts",
        "clients/tui/descriptor/fixtures/native-profile.json",
        "clients/tui/app/unit_tests/case.rs",
        "clients/tui/app/test_support/fixture.rs",
        "clients/tui/descriptor/generated/src/lib.rs",
        "clients/tui/descriptor/fixtures/src/lib.rs",
        "clients/tui/descriptor/lib.rs",
        "clients/tui/descriptor/src_extra/lib.rs",
        "clients_other/descriptor/src/lib.rs",
        "external/crate/clients/tui/descriptor/src/lib.rs",
    ] {
        assert_eq!(production_key(source).unwrap(), None, "{source}");
        assert_eq!(production_scope(source).unwrap(), None, "{source}");
    }
    for source in [
        "clients/../core/src/lib.rs",
        "clients/tui//src/lib.rs",
        "clients/",
    ] {
        assert!(production_key(source).is_err(), "{source}");
    }
}

#[test]
fn tooling_support_packages_and_test_directories_are_excluded() {
    for source in [
        "build_system/tool/unit_tests/case.rs",
        "build_system/tool/integration_tests/case.rs",
        "build_system/tool/src/edge_tests/case.rs",
        "build_system/tool/test_support/src/lib.rs",
        "build_system/tool_test_support/src/lib.rs",
        "build_system/tool/fixtures/src/lib.rs",
        "build_system/tool/generated/src/lib.rs",
    ] {
        assert_eq!(production_key(source).unwrap(), None, "{source}");
    }
}

#[test]
fn core_test_files_and_legacy_test_directories_cannot_dilute_production_totals() {
    for source in [
        "core/src/test/case.rs",
        "core/src/tests/case.rs",
        "core/crates/worker/src/test.rs",
        "core/crates/worker/src/test_state.rs",
        "core/crates/worker/src/tests_state.rs",
        "core/crates/worker/integration_tests/case.rs",
    ] {
        assert_eq!(production_key(source).unwrap(), None, "{source}");
        assert_eq!(production_scope(source).unwrap(), None, "{source}");
    }
}

#[test]
fn tooling_test_policy_implementations_are_production_code() {
    for source in [
        "build_system/architecture/checks/ownership/src/test_ownership.rs",
        "build_system/architecture/checks/ownership/src/test_rules.rs",
        "build_system/architecture/checks/ownership/src/tests_boundary.rs",
        "build_system/tool/src/testing.rs",
        "build_system/tool/src/tests.rs",
    ] {
        assert_eq!(
            production_scope(source).unwrap(),
            Some(ProductionScope::Tooling)
        );
        assert_eq!(production_key(source).unwrap().as_deref(), Some(source));
    }
}

#[test]
fn tooling_path_traversal_and_empty_components_fail_closed() {
    for source in [
        "build_system/tool/src/../../other/src/lib.rs",
        "build_system/tool//src/lib.rs",
        "build_system/tool/./src/lib.rs",
        "build_system/",
        "build_system/tool/src/",
    ] {
        assert!(production_key(source).is_err(), "{source}");
        assert!(production_scope(source).is_err(), "{source}");
    }
}
