use crate::constants::MAX_HANDWRITTEN_LINES;
use yolo_architecture_test_support::Fixture;

const OVERSIZED_FIXTURE: &str = "fixtures/oversized.jsonl";
const SENTINEL_LINE: &str = "{}";
const LIMIT_EXCESS: usize = 1;
const LINE_LIMIT_POLICY: &str = "handwritten source limit";

#[test]
fn jsonl_fixtures_are_scanned_as_handwritten_text() {
    let fixture = Fixture::new();
    let line_count = MAX_HANDWRITTEN_LINES + LIMIT_EXCESS;
    let contents = std::iter::repeat_n(SENTINEL_LINE, line_count)
        .collect::<Vec<_>>()
        .join("\n");
    fixture.write(OVERSIZED_FIXTURE, contents);
    let violations = crate::inspect(fixture.root()).unwrap();
    assert_eq!(violations.len(), LIMIT_EXCESS);
    assert!(violations[0].to_string().contains(LINE_LIMIT_POLICY));
}

#[test]
fn handwritten_capability_schemas_and_native_bridges_obey_source_size_limits() {
    for path in [
        "core/crates/transport/capnp/schema/oversized.capnp",
        "core/crates/transport/capnp/native/bridge/include/oversized.h",
        "core/crates/transport/capnp/native/bridge/include/module.modulemap",
    ] {
        let fixture = Fixture::new();
        fixture.write(
            path,
            "// declaration\n".repeat(MAX_HANDWRITTEN_LINES + LIMIT_EXCESS),
        );
        let violations = crate::inspect(fixture.root()).unwrap();
        assert_eq!(violations.len(), LIMIT_EXCESS, "{path}");
        assert!(violations[0].to_string().contains(LINE_LIMIT_POLICY));
    }
}
