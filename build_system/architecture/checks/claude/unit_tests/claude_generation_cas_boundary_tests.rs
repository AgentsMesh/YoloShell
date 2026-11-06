use super::*;
#[path = "claude_native_publication_boundary_tests.rs"]
mod native_publication;

#[test]
fn mutation_qualification_must_resolve_through_the_exact_parent_module() {
    for (prefix, valid) in [
        ("super::mutation::", true),
        ("super::super::mutation::", true),
        ("super::super::super::mutation::", true),
        ("", false),
        ("mutation::", false),
        ("crate::mutation::", false),
        ("super::other::", false),
        ("super.mutation::", false),
    ] {
        let fixture = Fixture::new();
        write_contract(&fixture);
        for path in SITES
            .iter()
            .map(|site| site.0)
            .collect::<std::collections::BTreeSet<_>>()
        {
            let source = std::fs::read_to_string(fixture.root().join(path)).unwrap();
            fixture.write(path, source.replace("super::mutation::", prefix));
        }
        let result = messages(&fixture);
        assert_eq!(result.is_empty(), valid, "{prefix}: {result:?}");
        if !valid {
            assert!(result.iter().any(|item| item.contains(POLICY_TEXT)));
        }
    }
}

#[test]
fn each_private_definition_token_and_the_implementation_count_is_required() {
    for declaration in [
        format!("struct {CLAUDE_GENERATION_MUTATION_TYPE};"),
        format!("pub(super) type {CLAUDE_GENERATION_MUTATION_TYPE} = Other;"),
        format!("pub(crate) struct {CLAUDE_GENERATION_MUTATION_TYPE};"),
        format!("pub[super] struct {CLAUDE_GENERATION_MUTATION_TYPE};"),
        format!("pub(super] struct {CLAUDE_GENERATION_MUTATION_TYPE};"),
        format!("pub[super) struct {CLAUDE_GENERATION_MUTATION_TYPE};"),
        format!("private(super) struct {CLAUDE_GENERATION_MUTATION_TYPE};"),
    ] {
        let fixture = Fixture::new();
        write_contract(&fixture);
        fixture.write(
            CLAUDE_GENERATION_MUTATION_PATH,
            format!("{declaration}\nimpl {CLAUDE_GENERATION_MUTATION_TYPE} {{}}"),
        );
        let result = messages(&fixture);
        assert!(
            result.iter().any(|item| item.contains(POLICY_TEXT)),
            "{declaration}: {result:?}"
        );
        assert!(
            result.iter().any(|item| item.contains(CONTRACT_TEXT)),
            "{declaration}: {result:?}"
        );
    }
    for implementations in [
        "".to_owned(),
        format!(
            "impl {CLAUDE_GENERATION_MUTATION_TYPE} {{}} impl {CLAUDE_GENERATION_MUTATION_TYPE} {{}} "
        ),
    ] {
        let fixture = Fixture::new();
        write_contract(&fixture);
        fixture.write(
            CLAUDE_GENERATION_MUTATION_PATH,
            format!("pub(super) struct {CLAUDE_GENERATION_MUTATION_TYPE}; {implementations}"),
        );
        assert!(
            messages(&fixture)
                .iter()
                .any(|item| item.contains(CONTRACT_TEXT))
        );
    }
}

#[test]
fn acquisition_requires_a_call_and_cannot_borrow_a_previous_function_context() {
    for occurrence in [
        format!("super::mutation::{CLAUDE_GENERATION_MUTATION_TYPE}"),
        format!(
            "super::mutation::{CLAUDE_GENERATION_MUTATION_TYPE}::{CLAUDE_GENERATION_MUTATION_ACQUIRE}"
        ),
        format!("super::mutation::{CLAUDE_GENERATION_MUTATION_TYPE}::other()"),
    ] {
        let fixture = Fixture::new();
        write_contract(&fixture);
        fixture.write(
            CLAUDE_GENERATION_STORE_PATH,
            format!("fn {CLAUDE_GENERATION_BEGIN_OWNER}() {{ {occurrence}; }}"),
        );
        assert!(
            messages(&fixture)
                .iter()
                .any(|item| item.contains(POLICY_TEXT)),
            "{occurrence}"
        );
    }
    for prefix in [
        format!("fn {CLAUDE_GENERATION_BEGIN_OWNER}();"),
        format!("fn {CLAUDE_GENERATION_BEGIN_OWNER}() {{ {{}} }}"),
    ] {
        let fixture = Fixture::new();
        write_contract(&fixture);
        fixture.write(CLAUDE_GENERATION_STORE_PATH, format!("{prefix} fn unrelated() {{ super::mutation::{CLAUDE_GENERATION_MUTATION_TYPE}::{CLAUDE_GENERATION_MUTATION_ACQUIRE}(); }}"));
        assert!(
            messages(&fixture)
                .iter()
                .any(|item| item.contains(POLICY_TEXT)),
            "{prefix}"
        );
    }
}
