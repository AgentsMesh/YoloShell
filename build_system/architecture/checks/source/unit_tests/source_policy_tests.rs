use super::{
    MISPLACED_TEST_SUPPORT_MESSAGE, RUST_SOURCE_TEST_MESSAGE, TEST_SUPPORT_TEST_MESSAGE,
    contains_test_declaration, inspect, is_handwritten, is_rust_test_source,
    misplaced_test_support, rust_implementation_source,
};
use std::path::Path;
use yolo_architecture_foundation::SourceFile;

#[test]
fn expect_drivers_are_handwritten_sources() {
    assert!(is_handwritten(Path::new(
        "plugins/web/client/scripts/session.exp"
    )));
    assert!(is_handwritten(Path::new(
        "core/crates/transport/capnp/schema/rpc.capnp"
    )));
    assert!(is_handwritten(Path::new(
        "core/crates/transport/capnp/native/bridge/include/yolo_native_value.h"
    )));
    assert!(is_handwritten(Path::new(
        "core/crates/transport/capnp/native/bridge/include/module.modulemap"
    )));
    assert!(is_handwritten(Path::new(".gitattributes")));
}

#[test]
fn test_support_is_separate_from_executable_test_directories() {
    for source in [
        "build_system/example/unit_tests/fixture.rs",
        "build_system/example/integration_tests/process_support.rs",
        "build_system/example/unit_tests/constants.rs",
    ] {
        assert!(misplaced_test_support(source), "{source}");
    }
    for source in [
        "build_system/example/test_support/fixture.rs",
        "build_system/example/unit_tests/test_support_policy_tests.rs",
    ] {
        assert!(!misplaced_test_support(source), "{source}");
    }
}

#[test]
fn rust_test_directories_follow_component_boundaries_and_suffixes() {
    for source in [
        "core/src/agent/test/helper.rs",
        "core/src/agent/tests/helper.rs",
        "core/src/agent/adversarial_tests/helper.rs",
        "core/crates/application/socket/guard/process/integration_tests/fixture.rs",
        "core/crates/application/socket/guard/process/test_support/fixture.rs",
    ] {
        assert!(is_rust_test_source(source), "{source}");
    }
}

#[test]
fn rust_production_directories_reject_test_near_matches() {
    for source in [
        "core/src/agent/contest/helper.rs",
        "core/src/agent/latest/helper.rs",
        "core/src/agent/tests_backup/helper.rs",
        "core/src/agent/adversarial_tests_extra/helper.rs",
    ] {
        assert!(!is_rust_test_source(source), "{source}");
    }
}

#[test]
fn rust_test_file_patterns_remain_component_independent() {
    for source in [
        "core/src/agent/test.rs",
        "core/src/agent/tests.rs",
        "core/src/agent/edge_test.rs",
        "core/src/agent/edge_tests.rs",
    ] {
        assert!(is_rust_test_source(source), "{source}");
    }
    assert!(!is_rust_test_source("core/src/agent/testing.rs"));
    assert!(!is_rust_test_source("core/src/agent/edge_tests.txt"));
}

#[test]
fn implementation_sources_exclude_nested_test_trees() {
    assert!(rust_implementation_source(
        "core/crates/transport/remote/src/backend.rs"
    ));
    assert!(rust_implementation_source(
        "core/crates/transport/remote/src/backend_tests/server.rs"
    ));
    assert!(rust_implementation_source(
        "build_system/architecture/syntax/src/parser.rs"
    ));
    assert!(!rust_implementation_source(
        "core/crates/runtime/engine/worker/unit_tests/src/helper.rs"
    ));
    assert!(!rust_implementation_source(
        "core/crates/transport/remote/integration_tests/server.rs"
    ));
}

#[test]
fn exact_rust_test_attributes_are_detected() {
    assert!(contains_test_declaration("#[test]\nfn check() {}\n"));
    assert!(contains_test_declaration(
        "#[cfg(target_os = \"macos\")] #[tokio::test]\nasync fn check() {}\n"
    ));
    assert!(contains_test_declaration(
        "#[tokio::test(flavor = \"multi_thread\")]\nasync fn check() {}\n"
    ));
    assert!(!contains_test_declaration(
        "#[cfg(test)]\n#[path = \"../unit_tests/check.rs\"]\nmod tests;\n"
    ));
}

#[test]
fn inspection_enforces_separate_production_tests_and_support_boundaries() {
    for (path, text, expected) in [
        (
            "core/crates/example/src/component.rs",
            "#[test] fn check() {}",
            Some(RUST_SOURCE_TEST_MESSAGE),
        ),
        (
            "core/crates/example/src/tests/component.rs",
            "fn helper() {}",
            Some(RUST_SOURCE_TEST_MESSAGE),
        ),
        (
            "build_system/example/unit_tests/fixture.rs",
            "fn helper() {}",
            Some(MISPLACED_TEST_SUPPORT_MESSAGE),
        ),
        (
            "build_system/example/test_support/fixture.rs",
            "#[test] fn check() {}",
            Some(TEST_SUPPORT_TEST_MESSAGE),
        ),
        (
            "build_system/example/test_support/fixture.rs",
            "fn helper() {}",
            None,
        ),
        (
            "core/crates/example/unit_tests/component.rs",
            "#[test] fn check() {}",
            None,
        ),
    ] {
        let file = SourceFile {
            absolute: path.into(),
            relative: path.into(),
            text: text.into(),
        };
        let mut violations = Vec::new();
        inspect(&file, &mut violations);
        let actual = violations
            .iter()
            .map(|value| value.message.as_str())
            .collect::<Vec<_>>();
        assert_eq!(actual, expected.into_iter().collect::<Vec<_>>(), "{path}");
    }
}
