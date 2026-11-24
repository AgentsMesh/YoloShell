use std::collections::BTreeMap;
use crate::claude_generation_constants::*;
use yolo_architecture_test_support::Fixture;
#[path = "claude_generation_cas_boundary_tests.rs"]
mod boundary_tests;

#[path = "../test_support/sites.rs"]
pub(super) mod sites;
pub(super) use sites::{ACQUIRE, BORROW, CONTRACT_TEXT, FUTURE_PATH, POLICY_TEXT, SITES};

pub(super) fn messages(fixture: &Fixture) -> Vec<String> {
    let files = yolo_architecture_test_support::load(fixture.root()).unwrap();
    let mut violations = Vec::new();
    crate::inspect(&files, &mut violations);
    violations
        .into_iter()
        .map(|violation| violation.to_string())
        .filter(|message| message.contains("Claude generation mutation"))
        .collect()
}

pub(super) fn write_contract(fixture: &Fixture) {
    fixture.write(
        CLAUDE_GENERATION_MUTATION_PATH,
        format!(
            "pub(super) struct {CLAUDE_GENERATION_MUTATION_TYPE}<'a> {{ value: &'a () }}\n\
             impl<'a> {CLAUDE_GENERATION_MUTATION_TYPE}<'a> {{}}\n"
        ),
    );
    let mut files = BTreeMap::<&str, String>::new();
    for (path, function, acquire) in SITES {
        let source = files.entry(path).or_default();
        if *acquire {
            source.push_str(&format!(
                "fn {function}() {{ super::mutation::{CLAUDE_GENERATION_MUTATION_TYPE}::\
                 {CLAUDE_GENERATION_MUTATION_ACQUIRE}(store, namespace); }}\n"
            ));
        } else {
            source.push_str(&format!(
                "fn {function}(mutation: &super::mutation::{CLAUDE_GENERATION_MUTATION_TYPE}<'_>) \
                 {{ consume(mutation); }}\n"
            ));
        }
    }
    for (path, source) in files {
        fixture.write(path, source);
    }
}

#[test]
fn accepts_the_exact_private_capability_graph() {
    let fixture = Fixture::new();
    write_contract(&fixture);
    assert!(messages(&fixture).is_empty());
}

#[test]
fn rejects_a_broader_or_moved_definition() {
    let fixture = Fixture::new();
    write_contract(&fixture);
    fixture.write(
        CLAUDE_GENERATION_MUTATION_PATH,
        format!(
            "pub(crate) struct {CLAUDE_GENERATION_MUTATION_TYPE};\n\
             impl {CLAUDE_GENERATION_MUTATION_TYPE} {{}}\n"
        ),
    );
    let result = messages(&fixture);
    assert!(result.iter().any(|item| item.contains(POLICY_TEXT)));
    assert!(result.iter().any(|item| item.contains(CONTRACT_TEXT)));

    let moved = Fixture::new();
    write_contract(&moved);
    moved.write(
        FUTURE_PATH,
        format!("pub(super) struct {CLAUDE_GENERATION_MUTATION_TYPE};\n"),
    );
    assert!(
        messages(&moved)
            .iter()
            .any(|item| item.contains(POLICY_TEXT))
    );
}

#[test]
fn rejects_a_deleted_capability_definition() {
    let fixture = Fixture::new();
    write_contract(&fixture);
    fixture.write(CLAUDE_GENERATION_MUTATION_PATH, "fn unrelated() {}\n");
    assert!(
        messages(&fixture)
            .iter()
            .any(|item| item.contains(CONTRACT_TEXT))
    );
}

#[test]
fn rejects_external_imports_holders_constructors_and_calls() {
    let fixture = Fixture::new();
    write_contract(&fixture);
    fixture.write(
        FUTURE_PATH,
        format!(
            "use super::mutation::{CLAUDE_GENERATION_MUTATION_TYPE} as Mutation;\n\
             struct Holder {{ mutation: {CLAUDE_GENERATION_MUTATION_TYPE}<'static> }}\n\
             fn construct() {{ let value = {CLAUDE_GENERATION_MUTATION_TYPE} {{}}; }}\n\
             fn call() {{ {CLAUDE_GENERATION_MUTATION_TYPE}::\
             {CLAUDE_GENERATION_MUTATION_ACQUIRE}(store, namespace); }}\n"
        ),
    );
    assert_eq!(
        messages(&fixture)
            .iter()
            .filter(|item| item.contains(POLICY_TEXT))
            .count(),
        4
    );
}

#[test]
fn rejects_unqualified_or_duplicate_owner_consumption() {
    let fixture = Fixture::new();
    write_contract(&fixture);
    fixture.write(
        FUTURE_PATH,
        format!(
            "fn bypass() {{ super::mutation::{CLAUDE_GENERATION_MUTATION_TYPE}::\
             {CLAUDE_GENERATION_MUTATION_ACQUIRE}(store, namespace); }}\n"
        ),
    );
    let result = messages(&fixture);
    assert!(result.iter().any(|item| item.contains(POLICY_TEXT)));

    let duplicate = Fixture::new();
    write_contract(&duplicate);
    let source = format!(
        "fn {CLAUDE_GENERATION_BEGIN_OWNER}() {{\n\
         super::mutation::{CLAUDE_GENERATION_MUTATION_TYPE}::\
         {CLAUDE_GENERATION_MUTATION_ACQUIRE}(store, namespace);\n\
         super::mutation::{CLAUDE_GENERATION_MUTATION_TYPE}::\
         {CLAUDE_GENERATION_MUTATION_ACQUIRE}(store, namespace);\n}}\n"
    );
    duplicate.write(CLAUDE_GENERATION_STORE_PATH, source);
    assert!(
        messages(&duplicate)
            .iter()
            .any(|item| item.contains(CONTRACT_TEXT))
    );
}

#[test]
fn rejects_a_value_parameter_without_a_capability_borrow() {
    let fixture = Fixture::new();
    write_contract(&fixture);
    fixture.write(
        CLAUDE_GENERATION_STORE_PATH,
        format!(
            "fn {CLAUDE_GENERATION_EXCHANGE_IN_OWNER}(other: &Thing, mutation: \
             super::mutation::{CLAUDE_GENERATION_MUTATION_TYPE}<'_>) {{ consume(mutation); }}\n"
        ),
    );
    assert!(
        messages(&fixture)
            .iter()
            .any(|item| item.contains(POLICY_TEXT))
    );
}
