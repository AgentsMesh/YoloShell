use crate::source::production_key;

#[test]
fn production_sources_accept_normalized_workspace_relative_paths() {
    let expected = Some("core/src/agent/detector.rs".to_owned());
    assert_eq!(
        production_key("core/src/agent/detector.rs").unwrap(),
        expected
    );
    assert_eq!(
        production_key("./core/src/agent/detector.rs").unwrap(),
        expected
    );
}

#[test]
fn sources_outside_production_roots_are_ignored() {
    for source in [
        "integration_tests/runtime/protocol.rs",
        "external/dependency/src/lib.rs",
        "external/dependency/core/src/lib.rs",
        "/tmp/execroot/_main/core/src/agent/detector.rs",
    ] {
        assert_eq!(production_key(source).unwrap(), None, "{source}");
    }
}

#[test]
fn test_only_source_patterns_are_ignored() {
    for source in [
        "core/src/worker/tests.rs",
        "core/src/protocol/test_sync.rs",
        "core/src/protocol/tests_sync.rs",
        "core/crates/wire/src/tests_sync.rs",
        "core/src/agent/detector_adversarial_tests.rs",
        "core/src/remote/backend_tests/server.rs",
        "core/src/socket_root_guard_process_tests/fixture.rs",
        "core/src/test_support/helper.rs",
    ] {
        assert_eq!(production_key(source).unwrap(), None, "{source}");
    }
}

#[test]
fn similarly_named_production_sources_are_preserved() {
    for source in [
        "core/src/testing.rs",
        "core/src/contest.rs",
        "core/src/coverage.rs",
        "core/src/agent/testimony.rs",
        "core/crates/wire/src/custom.rs",
    ] {
        assert!(production_key(source).unwrap().is_some(), "{source}");
    }
}

#[test]
fn traversal_and_empty_components_are_rejected() {
    for source in [
        "core/src/../secret.rs",
        "core/src/agent//detector.rs",
        "core/src/./detector.rs",
        "core/src/",
    ] {
        assert!(production_key(source).is_err(), "{source}");
    }
}
