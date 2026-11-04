use crate::constants::{MAX_HANDWRITTEN_LINES, joined};
use yolo_architecture_test_support::Fixture;

fn messages(fixture: &Fixture) -> Vec<String> {
    crate::inspect(fixture.root())
        .unwrap()
        .into_iter()
        .map(|violation| violation.to_string())
        .collect()
}

#[test]
fn nested_build_system_sources_cannot_escape_repository_policies() {
    let fixture = Fixture::new();
    let oversized = std::iter::repeat_n("let value = 1;", MAX_HANDWRITTEN_LINES + 1)
        .collect::<Vec<_>>()
        .join("\n");
    let retired = joined(&["con", "flux"]);
    let event = joined(&["writer", "_changed"]);
    let duration = joined(&["Duration::from_", "secs(5)"]);
    fixture.write("build_system/coverage/oversized.rs", oversized);
    fixture.write("build_system/coverage/legacy.js", "");
    fixture.write(
        "build_system/coverage/brand.rs",
        format!("// {retired}\n"),
    );
    fixture.write("build_system/coverage/build.sh", "cargo test\n");
    fixture.write(
        "core/src/constants.rs",
        format!("pub const EVENT: &str = \"{event}\";\n"),
    );
    fixture.write(
        "build_system/coverage/consumer.rs",
        format!("const LOCAL_EVENT: &str = \"{event}\";\n"),
    );
    fixture.write(
        "build_system/coverage/timing.rs",
        format!("consume({duration});\n"),
    );

    let result = messages(&fixture);
    assert_eq!(result.len(), 6, "{result:?}");
    for expected in [
        "handwritten source limit",
        "tooling is retired",
        "retired product identity",
        "execute Cargo",
        "centralized constant",
        "named constant",
    ] {
        assert!(
            result.iter().any(|entry| entry.contains(expected)),
            "{expected}: {result:?}"
        );
    }
}

#[test]
fn repository_root_coverage_output_remains_ignored() {
    let fixture = Fixture::new();
    let oversized = std::iter::repeat_n("let value = 1;", MAX_HANDWRITTEN_LINES + 1)
        .collect::<Vec<_>>()
        .join("\n");
    fixture.write("coverage/generated.rs", oversized);
    assert!(messages(&fixture).is_empty());
}

#[test]
fn repository_verification_snapshots_remain_ignored() {
    let fixture = Fixture::new();
    let oversized = std::iter::repeat_n("let value = 1;", MAX_HANDWRITTEN_LINES + 1)
        .collect::<Vec<_>>()
        .join("\n");
    fixture.write(".verification/snapshot/core/generated.rs", oversized);

    assert!(messages(&fixture).is_empty());
}
