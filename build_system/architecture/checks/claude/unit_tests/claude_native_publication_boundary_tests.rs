use super::*;
#[path = "../test_support/claude_native_publication_constants.rs"]
mod constants;
use self::constants::*;

fn acquire(function: &str, qualification: &str) -> String {
    format!(
        "fn {function}() {{ {qualification}{CLAUDE_GENERATION_MUTATION_TYPE}::{CLAUDE_GENERATION_MUTATION_ACQUIRE}(store, namespace); }}\n"
    )
}

#[test]
fn extracted_native_publisher_preserves_exact_owner_and_parent_qualification() {
    let fixture = Fixture::new();
    write_contract(&fixture);
    fixture.write(
        CLAUDE_NATIVE_PUBLICATION_PATH,
        acquire(CLAUDE_NATIVE_PUBLISH_OWNER, OWNER_QUALIFICATION),
    );
    assert!(messages(&fixture).is_empty());
}

#[test]
fn native_publisher_permission_does_not_extend_to_old_file_neighbor_or_other_function() {
    for (path, function) in [
        (CLAUDE_NATIVE_STORE_PATH, CLAUDE_NATIVE_PUBLISH_OWNER),
        (NEIGHBOR_PATH, CLAUDE_NATIVE_PUBLISH_OWNER),
        (CLAUDE_NATIVE_PUBLICATION_PATH, UNREGISTERED_FUNCTION),
    ] {
        let fixture = Fixture::new();
        write_contract(&fixture);
        let mut text = std::fs::read_to_string(fixture.root().join(path)).unwrap_or_default();
        text.push_str(&acquire(function, OWNER_QUALIFICATION));
        fixture.write(path, text);
        assert!(
            messages(&fixture)
                .iter()
                .any(|message| message.contains(POLICY_TEXT)),
            "{path}::{function}"
        );
    }
}

#[test]
fn native_publisher_must_remain_present_at_its_exact_enrolled_site() {
    let fixture = Fixture::new();
    write_contract(&fixture);
    fixture.write(CLAUDE_NATIVE_PUBLICATION_PATH, "fn unrelated() {}\n");
    assert!(
        messages(&fixture)
            .iter()
            .any(|message| message.contains(CONTRACT_TEXT))
    );
}

#[test]
fn native_publisher_cannot_substitute_absolute_or_unqualified_mutation_names() {
    for qualification in ["crate::claude::generation::mutation::", "mutation::", ""] {
        let fixture = Fixture::new();
        write_contract(&fixture);
        fixture.write(
            CLAUDE_NATIVE_PUBLICATION_PATH,
            acquire(CLAUDE_NATIVE_PUBLISH_OWNER, qualification),
        );
        let result = messages(&fixture);
        assert!(result.iter().any(|message| message.contains(POLICY_TEXT)));
        assert!(result.iter().any(|message| message.contains(CONTRACT_TEXT)));
    }
}
