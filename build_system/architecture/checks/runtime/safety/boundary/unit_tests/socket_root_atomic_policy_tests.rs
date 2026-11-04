use yolo_architecture_policy_contracts::constants::{
    SESSION_MARKER_CAS_WRITER, SOCKET_ROOT_BOUNDARY_WRITER, SOCKET_ROOT_ENTRY_WRITER,
};
use yolo_architecture_test_support::Fixture;

const REMOVED_OWNER_PATH: &str =
    "core/crates/persistence/paths/src/socket_namespace_cleanup_intent.rs";
const DEFINITION_PATH: &str = "core/crates/persistence/paths/src/durable/durable_json_guarded.rs";
const EXPORT_PATH: &str = "core/crates/persistence/paths/src/mod.rs";
const FUTURE_PATH: &str = "core/crates/persistence/paths/src/socket/namespace/socket_namespace_future.rs";
const POLICY_TEXT: &str = "socket-root generic atomic publication is retired";

fn messages(fixture: &Fixture) -> Vec<String> {
    crate::inspect(fixture.root())
        .unwrap()
        .into_iter()
        .map(|violation| violation.to_string())
        .collect()
}

#[test]
fn retired_adapter_has_no_permitted_definition_export_or_previous_owner() {
    for (path, text) in [
        (
            DEFINITION_PATH,
            format!(
                "pub(crate) fn {SOCKET_ROOT_BOUNDARY_WRITER}<T>() {{}}\npub(super) fn {SESSION_MARKER_CAS_WRITER}<T>() {{}}"
            ),
        ),
        (
            EXPORT_PATH,
            format!("pub(crate) use durable_json_guarded::{SOCKET_ROOT_BOUNDARY_WRITER};"),
        ),
        (
            REMOVED_OWNER_PATH,
            format!(
                "fn publish() {{ crate::paths::{SOCKET_ROOT_BOUNDARY_WRITER}(entry, value); }}"
            ),
        ),
        (
            FUTURE_PATH,
            format!(
                "fn publish() {{ crate::paths::{SOCKET_ROOT_BOUNDARY_WRITER}(entry, value); }}"
            ),
        ),
    ] {
        let fixture = Fixture::new();
        fixture.write(path, text);
        let result = messages(&fixture);
        assert_eq!(result.len(), 1, "{path}");
        assert!(result[0].contains(POLICY_TEXT));
    }
}

#[test]
fn aliases_and_generic_socket_namespace_writers_are_rejected() {
    let fixture = Fixture::new();
    fixture.write(
        FUTURE_PATH,
        format!(
            "use crate::paths::{SOCKET_ROOT_BOUNDARY_WRITER} as publish;\nuse crate::paths::{SOCKET_ROOT_ENTRY_WRITER} as replace;\nuse crate::paths::atomic_write_json as persist;\n"
        ),
    );
    let result = messages(&fixture);
    assert_eq!(result.len(), 3);
    assert!(result.iter().all(|message| message.contains(POLICY_TEXT)));
}

#[test]
fn comments_strings_and_unrelated_durable_writers_are_not_publication_authority() {
    let fixture = Fixture::new();
    fixture.write(
        FUTURE_PATH,
        format!("// {SOCKET_ROOT_BOUNDARY_WRITER}(ignored)\nconst NOTE: &str = \"atomic_write_json(ignored)\";\n"),
    );
    fixture.write(
        "core/crates/runtime/engine/worker/src/state.rs",
        "fn persist() { atomic_write_json(path, value); }\n",
    );
    assert!(messages(&fixture).is_empty());
}
