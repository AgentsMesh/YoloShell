use crate::RUST_COVERAGE_DECLARATIONS;
use yolo_architecture_source_policy::constants::MAX_HANDWRITTEN_LINES;
use yolo_architecture_test_support::Fixture;

fn messages(fixture: &Fixture) -> Vec<String> {
    crate::inspect(fixture.root())
        .unwrap()
        .into_iter()
        .map(|violation| violation.to_string())
        .collect()
}

fn repeated(line: &str, count: usize) -> String {
    std::iter::repeat_n(line, count)
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn vendored_web_assets_do_not_consume_the_handwritten_limit() {
    let fixture = Fixture::new();
    let rejected_line_count = MAX_HANDWRITTEN_LINES + 1;
    fixture.write(
        "plugins/web/client/vendor/xterm.css",
        repeated("a", rejected_line_count),
    );
    fixture.write(
        "plugins/web/client/vendor/manifest.json",
        repeated("{}", rejected_line_count),
    );
    fixture.write(
        "plugins/web/client/vendor/xterm.js",
        repeated("var value = 1;", rejected_line_count),
    );
    let result = messages(&fixture);
    assert!(result.is_empty(), "{result:?}");

    fixture.write(
        "plugins/web/client/styles-large.css",
        repeated("a", rejected_line_count),
    );
    let result = messages(&fixture);
    assert_eq!(result.len(), 1, "{result:?}");
    assert!(result[0].contains("handwritten source limit"));
}

#[test]
fn rejects_oversized_handwritten_sources_and_docs() {
    let fixture = Fixture::new();
    let rejected_line_count = MAX_HANDWRITTEN_LINES + 1;
    fixture.write(
        "core/src/large.rs",
        repeated("let value = 1;", rejected_line_count),
    );
    fixture.write("docs/large.md", repeated("text", rejected_line_count));
    fixture.write(
        "plugins/macos/Resources/large.json",
        repeated("{}", rejected_line_count),
    );
    let configuration = RUST_COVERAGE_DECLARATIONS
        .iter()
        .copied()
        .chain(std::iter::repeat_n(
            "build --flag",
            rejected_line_count - RUST_COVERAGE_DECLARATIONS.len(),
        ))
        .collect::<Vec<_>>()
        .join("\n");
    fixture.write(".bazelrc", configuration);
    fixture.write("Cargo.lock", repeated("generated", rejected_line_count));
    fixture.write(
        "MODULE.bazel.lock",
        repeated("generated", rejected_line_count),
    );

    let result = messages(&fixture);
    assert_eq!(result.len(), 4, "{result:?}");
    assert!(
        result
            .iter()
            .all(|message| { message.contains(&format!("{rejected_line_count} lines")) })
    );
}
