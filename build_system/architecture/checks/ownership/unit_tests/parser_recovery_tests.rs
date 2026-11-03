use super::constants::{ISOLATED_RUST_TEST_RULE, RUST_TEST_RULE};
use super::support::{inspect, messages, source};

const PACKAGE: &str = "build_system/example";
const UNIT_SOURCE: &str = "unit_tests/case.rs";
const BUILD_FILE: &str = "build_system/example/BUILD.bazel";
const COMPREHENSION: &str =
    "[dependency for dependency in DEPS if dependency not in [COMPONENT, select({KEY: []})]]";

fn report(build: &str) -> String {
    messages(&inspect(vec![
        source(BUILD_FILE, build),
        source(&format!("{PACKAGE}/{UNIT_SOURCE}"), "#[test] fn case() {}"),
    ]))
}

#[test]
fn dependency_comprehensions_preserve_source_ownership_in_either_attribute_order() {
    for rule in [RUST_TEST_RULE, ISOLATED_RUST_TEST_RULE] {
        for attributes in [
            format!("srcs = [\"{UNIT_SOURCE}\"], deps = {COMPREHENSION} + [EXTRA]"),
            format!("deps = {COMPREHENSION}, srcs = [\"{UNIT_SOURCE}\"]"),
        ] {
            let result = report(&format!("{rule}(name = \"unit\", {attributes})"));
            assert!(result.is_empty(), "{result}");
        }
    }
}

#[test]
fn source_comprehensions_do_not_claim_their_literal_element() {
    let result = report(&format!(
        "rust_test(name = \"unit\", srcs = [\"{UNIT_SOURCE}\" for item in UNKNOWN])"
    ));
    assert!(result.contains("must be statically resolvable"), "{result}");
    assert!(result.contains("must belong to at least one"), "{result}");
}

#[test]
fn incomplete_comprehensions_cannot_claim_preceding_sources() {
    for ending in ["", "[", "{", "(", "[)", "{]", "(}"] {
        let result = report(&format!(
            "rust_test(name = \"unit\", srcs = [\"{UNIT_SOURCE}\"], deps = [x for x in {ending}"
        ));
        assert!(result.contains("must belong to at least one"), "{result}");
    }
}

#[test]
fn missing_attribute_separator_cannot_claim_preceding_sources() {
    let result = report(&format!(
        "rust_test(name = \"unit\", srcs = [\"{UNIT_SOURCE}\"] deps = [])"
    ));
    assert!(result.contains("must belong to at least one"), "{result}");
}

#[test]
fn nested_comprehensions_cannot_create_forged_targets() {
    let result = report(&format!(
        "rust_test(name = \"unit\", deps = [x for x in [rust_test(\
         name = \"forged\", srcs = [\"{UNIT_SOURCE}\"]) for x in DEPS]], srcs = [])"
    ));
    assert!(result.contains("must belong to at least one"), "{result}");
}

#[test]
fn bare_rule_names_and_unclosed_calls_or_lists_cannot_claim_ownership() {
    for text in [
        RUST_TEST_RULE,
        "rust_test(",
        "rust_test(name = \"unit\",",
        "[",
    ] {
        let tokens = yolo_architecture_syntax::cargo_starlark_tokens::tokenize(text);
        if text != RUST_TEST_RULE {
            assert!(
                crate::expression_parser::ExpressionParser::new(&tokens, 0)
                    .parse()
                    .is_none(),
                "{text}"
            );
        }
        assert!(
            crate::parser::document(&source(BUILD_FILE, text))
                .rules
                .is_empty(),
            "{text}"
        );
    }
}

#[test]
fn load_imports_never_promote_dynamic_positional_or_named_symbols() {
    let document = crate::parser::document(&source(
        BUILD_FILE,
        "load(\":rules.bzl\", DYNAMIC, alias = DYNAMIC, valid = \"rust_library\")",
    ));
    assert_eq!(document.imports.len(), 1);
    assert_eq!(
        document.imports["valid"],
        format!("{PACKAGE}/rules.bzl#rust_library")
    );
}
