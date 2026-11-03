use crate::constants::{
    DETACHED_STDERR_DEFINITION, DETACHED_STDERR_INHERIT_PARTS, DETACHED_STDERR_OWNER,
};
use yolo_architecture_foundation::text::joined;
use yolo_architecture_test_support::Fixture;

fn messages(fixture: &Fixture) -> Vec<String> {
    crate::inspect(fixture.root())
        .unwrap()
        .into_iter()
        .map(|violation| violation.to_string())
        .collect()
}

fn inherited_standard_error() -> String {
    joined(DETACHED_STDERR_INHERIT_PARTS)
}

#[test]
fn accepts_the_single_session_worker_inheritance_point() {
    let fixture = Fixture::new();
    let marker = inherited_standard_error();
    fixture.write(
        DETACHED_STDERR_DEFINITION,
        format!(
            "enum Mode {{ {} }}\nmatch mode {{ {marker} => () }}\n",
            variant()
        ),
    );
    fixture.write(
        DETACHED_STDERR_OWNER,
        format!("spawn_detached({marker});\n"),
    );
    assert!(messages(&fixture).is_empty());
}

#[test]
fn rejects_inheritance_from_other_production_and_test_sources() {
    let fixture = Fixture::new();
    let source = format!("spawn_detached({});\n", inherited_standard_error());
    fixture.write("core/src/relay/spawn.rs", &source);
    fixture.write("core/crates/runtime/engine/process/src/tests.rs", source);
    fixture.write(
        "core/src/remote/spawn.rs",
        format!(
            "use process::{} as Mode;\nspawn_detached({});\n",
            DETACHED_STDERR_INHERIT_PARTS[0],
            joined(&["Mode", "::", "In", "herit"]),
        ),
    );
    let result = messages(&fixture);
    assert_eq!(result.len(), 3);
    assert_eq!(
        result
            .iter()
            .filter(|message| message.contains("reserved for the session worker"))
            .count(),
        3
    );
}

#[test]
fn rejects_missing_or_duplicate_inheritance_at_the_owner() {
    let missing = Fixture::new();
    missing.write(DETACHED_STDERR_OWNER, "spawn_detached(Null);\n");
    assert_eq!(messages(&missing).len(), 1);

    let duplicate = Fixture::new();
    let marker = inherited_standard_error();
    duplicate.write(
        DETACHED_STDERR_OWNER,
        format!("spawn_detached({marker});\nspawn_detached({marker});\n"),
    );
    assert_eq!(messages(&duplicate).len(), 1);

    let malformed_definition = Fixture::new();
    malformed_definition.write(
        DETACHED_STDERR_DEFINITION,
        format!("enum Mode {{ {} }}\n", variant()),
    );
    assert_eq!(messages(&malformed_definition).len(), 1);
}

#[test]
fn comments_and_strings_cannot_satisfy_required_inheritance_counts() {
    let owner = Fixture::new();
    let marker = inherited_standard_error();
    owner.write(
        DETACHED_STDERR_OWNER,
        format!("// {marker}\nconst NOTE: &str = \"{marker}\";\n"),
    );
    assert_eq!(messages(&owner).len(), 1);

    let definition = Fixture::new();
    definition.write(
        DETACHED_STDERR_DEFINITION,
        format!(
            "enum Mode {{}}\n/* {marker} */\nconst NOTE: &str = \"{}\";\n",
            variant()
        ),
    );
    assert_eq!(messages(&definition).len(), 1);
}

#[test]
fn comments_and_strings_cannot_forge_inheritance_in_other_sources() {
    let fixture = Fixture::new();
    let marker = inherited_standard_error();
    fixture.write(
        "core/src/relay/notes.rs",
        format!("// {marker}\nconst NOTE: &str = \"{marker}\";\n"),
    );
    assert!(messages(&fixture).is_empty());
}

fn variant() -> String {
    joined(&["In", "herit"])
}
