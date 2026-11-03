use crate::source_path::{is_production_rust_source, is_rust_test_source};

#[test]
fn classifies_test_directories_and_file_names() {
    for path in [
        "core/src/unit_tests/wiring.rs",
        "core/src/integration_tests/flow.rs",
        "core/src/test_support/fixture.rs",
        "core/src/agent/tests.rs",
        "core/src/agent/edge_tests.rs",
        "core/src/agent/test.rs",
    ] {
        assert!(is_rust_test_source(path), "{path}");
        assert!(!is_production_rust_source(path), "{path}");
    }
}

#[test]
fn keeps_production_sources() {
    for path in [
        "core/src/agent/testing.rs",
        "core/src/agent/contest.rs",
    ] {
        assert!(!is_rust_test_source(path), "{path}");
        assert!(is_production_rust_source(path), "{path}");
    }
    assert!(!is_rust_test_source("core/src/agent/edge_tests.txt"));
}
