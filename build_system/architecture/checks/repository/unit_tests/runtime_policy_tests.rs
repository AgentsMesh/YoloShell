use crate::constants::joined;
use yolo_architecture_source_policy::constants::MAX_HANDWRITTEN_LINES;
use yolo_architecture_test_support::Fixture;

fn messages(fixture: &Fixture) -> Vec<String> {
    crate::inspect(fixture.root())
        .unwrap()
        .into_iter()
        .map(|violation| violation.to_string())
        .collect()
}

#[test]
fn rejects_distributed_semantic_and_timing_values() {
    let fixture = Fixture::new();
    let state = joined(&["controlled", "_elsewhere"]);
    let timing = joined(&["30_", "000"]);
    fixture.write(
        "core/src/service.rs",
        format!("let state = \"{state}\";\nlet timeout = {timing};\n"),
    );
    let result = messages(&fixture);
    assert_eq!(result.len(), 2);
    assert!(
        result
            .iter()
            .any(|entry| entry.contains("centralized constant"))
    );
    assert!(result.iter().any(|entry| entry.contains("named constant")));
}

#[test]
fn rejects_native_window_focus_from_default_tests() {
    let fixture = Fixture::new();
    let actions = [
        joined(&["acti", "vate"]),
        joined(&["bring", "ToFront"]),
        joined(&["makeKey", "AndOrderFront"]),
        joined(&["show", "Window"]),
        joined(&["order", "Front"]),
        joined(&["orderFront", "Regardless"]),
        joined(&["make", "Key"]),
        joined(&["make", "Main"]),
        joined(&["become", "Key"]),
        joined(&["become", "Main"]),
        joined(&["demini", "aturize"]),
        joined(&["un", "hide"]),
        joined(&["launch", "Application"]),
        joined(&["open", "Application"]),
    ];
    fixture.write(
        "integration_tests/WindowTests.swift",
        actions
            .iter()
            .map(|action| format!("window.{action}()\n"))
            .collect::<String>(),
    );
    let result = messages(&fixture);
    assert_eq!(result.len(), actions.len());
    assert!(
        result
            .iter()
            .all(|entry| entry.contains("must not activate or focus"))
    );
}

#[test]
fn permits_non_call_activation_language_in_default_tests() {
    let fixture = Fixture::new();
    let property = joined(&["acti", "vationPolicy"]);
    let assertion = joined(&["neverActi", "vates"]);
    fixture.write(
        "integration_tests/ApplicationLaunchModeTests.swift",
        format!("let policy = application.{property}\nfunc {assertion}() {{}}\n"),
    );
    assert!(messages(&fixture).is_empty());
}

#[test]
fn permits_serde_case_directives_but_not_business_wire_renames() {
    let fixture = Fixture::new();
    let case = joined(&["snake", "_case"]);
    let state = joined(&["controlled", "_elsewhere"]);
    fixture.write(
        "core/src/protocol.rs",
        format!("pub const CASE: &str = \"{case}\";\npub const STATE: &str = \"{state}\";\n"),
    );
    fixture.write(
        "core/src/model.rs",
        format!(
            "#[serde(rename_all = \"{case}\")]\nenum State {{ Ready }}\n#[serde(rename = \"{state}\")]\nstruct Wire;\n"
        ),
    );
    let result = messages(&fixture);
    assert_eq!(result.len(), 1);
    assert!(result[0].contains(&state));
}

#[test]
fn permits_serde_bytes_adapter_but_not_an_ordinary_literal() {
    let fixture = Fixture::new();
    let adapter = joined(&["serde", "_bytes"]);
    fixture.write(
        "core/src/protocol.rs",
        format!("pub const ADAPTER: &str = \"{adapter}\";\n"),
    );
    fixture.write(
        "core/src/model.rs",
        format!(
            "#[serde(\n  default,\n  with = \"{adapter}\"\n)]\nstruct Blob;\nconst ACCIDENTAL: &str = \"{adapter}\";\n"
        ),
    );
    let result = messages(&fixture);
    assert_eq!(result.len(), 1);
    assert!(result[0].contains(&adapter));
}

#[test]
fn scans_source_files_exposed_as_runfiles_links() {
    let fixture = Fixture::new();
    fixture.write(
        ".build/generated.rs",
        std::iter::repeat_n("let value = 1;", MAX_HANDWRITTEN_LINES + 1)
            .collect::<Vec<_>>()
            .join("\n"),
    );
    let link = fixture.root().join("core/src/linked.rs");
    std::fs::create_dir_all(link.parent().unwrap()).unwrap();
    std::os::unix::fs::symlink(fixture.root().join(".build/generated.rs"), link).unwrap();
    let result = messages(&fixture);
    assert_eq!(result.len(), 1);
    assert!(result[0].contains("handwritten source limit"));
}

#[test]
fn rejects_the_portable_pty_writer_with_its_blocking_destructor() {
    let fixture = Fixture::new();
    let call = joined(&["master.take_", "writer().unwrap()"]);
    fixture.write(
        "core/crates/runtime/engine/worker/src/runtime.rs",
        format!("let writer = {call};\n"),
    );
    let result = messages(&fixture);
    assert_eq!(result.len(), 1);
    assert!(result[0].contains("bounded PTY input writer"));
}
