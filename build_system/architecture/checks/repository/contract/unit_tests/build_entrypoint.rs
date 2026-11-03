use super::*;
use yolo_architecture_test_support::Fixture;

const MACRO_FILE: &str = "build_system/bazel/rust/current_tests.bzl";

fn violations(path: &str, source: &str) -> Vec<Violation> {
    let fixture = Fixture::new();
    fixture.write(path, source);
    let files = yolo_architecture_test_support::load(fixture.root()).unwrap();
    let mut result = Vec::new();
    for file in files {
        inspect(&file, &mut result);
    }
    result
}

#[test]
fn moved_target_names_cannot_return_as_aliases_or_wrapped_rules() {
    for (package, names) in [
        (CORE_PACKAGE, CORE_FORWARDERS),
        (INTEGRATION_PACKAGE, INTEGRATION_FORWARDERS),
    ] {
        for name in names {
            for rule in ["alias", "native.alias", "wrapped_rule"] {
                let source = format!("{rule}(actual='//owner:current', name='{name}')");
                assert!(!violations(&format!("{package}/{BUILD_FILE}"), &source).is_empty());
            }
        }
    }
}

#[test]
fn references_to_historical_labels_are_rejected_outside_their_package() {
    for (package, names) in [
        (CORE_PACKAGE, CORE_FORWARDERS),
        (INTEGRATION_PACKAGE, INTEGRATION_FORWARDERS),
    ] {
        for name in names {
            let source = format!("TARGETS = ['//{package}:{name}']");
            assert!(!violations(MACRO_FILE, &source).is_empty());
            let local = format!("test_suite(name='tests', tests=[':{name}'])");
            assert!(!violations(&format!("{package}/{BUILD_FILE}"), &local).is_empty());
        }
    }
}

#[test]
fn retired_macro_sources_cannot_be_restored_even_when_empty() {
    for path in RETIRED_MACRO_FILES {
        assert!(!violations(path, "").is_empty());
    }
}

#[test]
fn owner_targets_current_product_entrypoints_and_comments_are_accepted() {
    for (path, source) in [
        (
            "core/BUILD.bazel",
            "alias(name='yolo', actual='//core/crates/application/yolo:yolo')",
        ),
        (
            "core/BUILD.bazel",
            "alias(name='yolo-adversarial', actual='//core/crates/application/yolo:yolo-adversarial')",
        ),
        (
            "integration_tests/runtime/BUILD.bazel",
            "isolated_rust_test(name='runtime_resilience_test')",
        ),
        (
            "integration_tests/runtime/protocol/BUILD.bazel",
            "isolated_rust_test(name='runtime_protocol_test')",
        ),
        (
            MACRO_FILE,
            "TARGETS = ['//integration_tests/runtime/protocol:runtime_protocol_test', '//build_system/bazel/rust/fixtures:codex_version_fixture']",
        ),
        (
            MACRO_FILE,
            "# alias(name='runtime_protocol_test', actual='//core:runtime_protocol_test')",
        ),
        ("docs/validation.md", "//core:runtime_protocol_test"),
    ] {
        assert!(violations(path, source).is_empty(), "{path}: {source}");
    }
}
