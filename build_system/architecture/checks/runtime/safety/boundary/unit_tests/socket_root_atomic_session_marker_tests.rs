use yolo_architecture_policy_contracts::constants::{
    SESSION_MARKER_CAS_OWNER_FUNCTION, SESSION_MARKER_CAS_WRITER,
};
use yolo_architecture_test_support::Fixture;

const DEFINITION_PATH: &str = "core/crates/persistence/paths/src/durable/durable_json_guarded.rs";
const EXPORT_PATH: &str = "core/crates/persistence/paths/src/mod.rs";
const SESSION_OWNER: &str = "core/crates/persistence/paths/src/resume.rs";
const POLICY_TEXT: &str = "session marker CAS publication";
const DEFINITION_TEXT: &str = "paths-private definition";
const OWNER_TEXT: &str = "exactly one session marker CAS publication";

fn messages(fixture: &Fixture) -> Vec<String> {
    crate::inspect(fixture.root())
        .unwrap()
        .into_iter()
        .map(|violation| violation.to_string())
        .collect()
}

fn write_contract(fixture: &Fixture, definition: &str, owner: &str) {
    fixture.write(DEFINITION_PATH, definition);
    fixture.write(SESSION_OWNER, owner);
}

fn definition() -> String {
    format!("pub(super) fn {SESSION_MARKER_CAS_WRITER}<T>() {{}}")
}

fn owner_call() -> String {
    format!(
        "fn {SESSION_MARKER_CAS_OWNER_FUNCTION}() {{\n    super::guarded::{SESSION_MARKER_CAS_WRITER}(entry);\n}}\n"
    )
}

#[test]
fn accepts_the_single_fixed_session_marker_publication() {
    let fixture = Fixture::new();
    write_contract(&fixture, &definition(), &owner_call());
    assert!(messages(&fixture).is_empty());
}

#[test]
fn rejects_moved_and_duplicated_session_marker_calls() {
    let moved = Fixture::new();
    let moved_owner = format!("fn other() {{ {SESSION_MARKER_CAS_WRITER}(entry); }}\n");
    write_contract(&moved, &definition(), &moved_owner);
    let moved_messages = messages(&moved);
    assert_eq!(moved_messages.len(), 2);
    assert!(moved_messages.iter().any(|item| item.contains(POLICY_TEXT)));
    assert!(moved_messages.iter().any(|item| item.contains(OWNER_TEXT)));

    let duplicate = Fixture::new();
    let call = format!("{SESSION_MARKER_CAS_WRITER}(entry);\n");
    let duplicate_owner = format!("fn {SESSION_MARKER_CAS_OWNER_FUNCTION}() {{\n{call}{call}}}\n");
    write_contract(&duplicate, &definition(), &duplicate_owner);
    let duplicate_messages = messages(&duplicate);
    assert_eq!(duplicate_messages.len(), 1);
    assert!(duplicate_messages[0].contains(OWNER_TEXT));
}

#[test]
fn rejects_broad_definition_alias_and_export_shapes() {
    let fixture = Fixture::new();
    let broad = format!("pub(crate) fn {SESSION_MARKER_CAS_WRITER}<T>() {{}}");
    write_contract(&fixture, &broad, &owner_call());
    fixture.write(
        EXPORT_PATH,
        format!("pub(crate) use durable_json_guarded::{SESSION_MARKER_CAS_WRITER};\n"),
    );
    fixture.write(
        "core/crates/persistence/paths/src/socket/namespace/socket_namespace_future.rs",
        format!("use crate::paths::{SESSION_MARKER_CAS_WRITER} as replace;\n"),
    );
    fixture.write(
        "core/crates/runtime/engine/worker/src/state.rs",
        format!("fn mutate() {{ {SESSION_MARKER_CAS_WRITER}(entry); }}\n"),
    );
    let result = messages(&fixture);
    assert_eq!(result.len(), 5);
    assert!(result.iter().any(|item| item.contains(DEFINITION_TEXT)));
    assert_eq!(
        result
            .iter()
            .filter(|item| item.contains(POLICY_TEXT))
            .count(),
        4
    );
}
