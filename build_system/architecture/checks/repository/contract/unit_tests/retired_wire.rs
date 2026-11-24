use super::*;
use yolo_architecture_test_support::Fixture;

#[path = "../test_support/retired_wire_constants.rs"]
mod constants;

const PRODUCTION: &str = "core/crates/runtime/engine/worker/src/dispatch.rs";
const UNIT: &str = "core/crates/runtime/engine/worker/unit_tests/rejection.rs";
const INTEGRATION: &str = "core/crates/runtime/engine/worker/integration_tests/rejection.rs";
const SUPPORT: &str = "core/crates/runtime/engine/worker/test_support/rejection.rs";

fn inspect_source(path: &str, source: &str) -> Vec<Violation> {
    let fixture = Fixture::new();
    fixture.write(path, source);
    let files = yolo_architecture_test_support::load(fixture.root()).unwrap();
    let mut violations = Vec::new();
    for file in files {
        inspect(&file, &mut violations);
    }
    violations
}

#[test]
fn old_symbols_cannot_return_as_declarations_dispatch_or_import_aliases() {
    for name in RETIRED_IDENTIFIERS {
        for source in [
            format!("pub struct {name};"),
            format!("use old::{name} as Current;"),
            format!("match method {{ protocol::{name} => invoke(), _ => reject() }}"),
        ] {
            assert!(!inspect_source(PRODUCTION, &source).is_empty(), "{source}");
        }
    }
}

#[test]
fn renamed_constants_and_direct_method_dispatch_do_not_hide_retired_values() {
    for value in RETIRED_WIRE_VALUES {
        for source in [
            format!("const RENAMED: &str = \"{value}\";"),
            format!("match method {{ \"{value}\" => invoke(), _ => reject() }}"),
            format!("client.request(\"{value}\", payload)"),
        ] {
            assert!(!inspect_source(PRODUCTION, &source).is_empty(), "{source}");
        }
    }
}

#[test]
fn raw_byte_and_escaped_literals_receive_the_same_wire_check() {
    for source in [
        "const NAME: &str = r#\"resume_session\"#;",
        "const NAME: &[u8] = br#\"ack_output\"#;",
        "const NAME: &[u8] = b\"detach_session\";",
        "const NAME: &str = \"resume\\x5fsession\";",
        "const NAME: &str = \"writer\\u{5f}lease\";",
    ] {
        assert!(!inspect_source(PRODUCTION, source).is_empty(), "{source}");
    }
}

#[test]
fn exact_current_names_internal_functions_and_comments_remain_valid() {
    assert!(inspect_source(PRODUCTION, constants::CURRENT_SOURCE).is_empty());
}

#[test]
fn separate_rejection_tests_keep_old_literals_without_exposing_production_paths() {
    let source = "struct TerminateRequest; const OLD: &str = \"resume_session\";";
    for path in [
        UNIT,
        INTEGRATION,
        SUPPORT,
        "plugins/macos/Sources/Example/Wire.swift",
    ] {
        assert!(inspect_source(path, source).is_empty(), "{path}");
    }
    assert!(!inspect_source(PRODUCTION, source).is_empty());
}
