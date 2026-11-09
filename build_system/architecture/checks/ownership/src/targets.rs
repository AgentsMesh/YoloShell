use crate::Repository;
use crate::constants::{
    ISOLATED_RUST_TEST_RULE, NAME_ATTRIBUTE, RUST_BINARY_RULE, RUST_LIBRARY_RULE,
    RUST_STATIC_LIBRARY_RULE, RUST_TEST_RULE, SOURCES_ATTRIBUTE, TEST_ONLY_ATTRIBUTE, TRUE_LITERAL,
};
use crate::model::{Expression, RuleCall, Target};

pub fn collect(repository: &Repository) -> Vec<Target> {
    collect_matching(repository, production_target)
}

pub fn collect_tests(repository: &Repository) -> Vec<Target> {
    collect_matching(repository, test_target)
}

pub fn collect_test_helpers(repository: &Repository) -> Vec<Target> {
    collect_matching(repository, |rule| {
        production_rule(&rule.name) && truthy(rule.attributes.get(TEST_ONLY_ATTRIBUTE))
    })
}

fn collect_matching(repository: &Repository, selected: fn(&RuleCall) -> bool) -> Vec<Target> {
    let mut targets = Vec::new();
    for document in repository
        .documents()
        .filter(|document| crate::paths::is_build_file(&document.path))
    {
        for rule in document.rules.iter().filter(|rule| selected(rule)) {
            let Some(expression) = rule.attributes.get(SOURCES_ATTRIBUTE) else {
                continue;
            };
            let evaluated =
                repository.evaluate(&document.path, &document.package, rule.line, expression);
            let name = string(rule.attributes.get(NAME_ATTRIBUTE)).unwrap_or(&rule.name);
            targets.push(Target {
                id: format!("{}:{}:{name}", document.path, rule.line),
                package: document.package.clone(),
                path: document.path.clone(),
                line: rule.line,
                sources: evaluated.paths,
                source_labels: evaluated.labels,
                sources_unresolved: evaluated.unresolved,
            });
        }
    }
    targets
}

fn production_rule(name: &str) -> bool {
    matches!(
        name,
        RUST_LIBRARY_RULE | RUST_STATIC_LIBRARY_RULE | RUST_BINARY_RULE
    )
}

fn test_rule(name: &str) -> bool {
    matches!(name, RUST_TEST_RULE | ISOLATED_RUST_TEST_RULE)
}

fn production_target(rule: &RuleCall) -> bool {
    production_rule(&rule.name) && !truthy(rule.attributes.get(TEST_ONLY_ATTRIBUTE))
}

fn test_target(rule: &RuleCall) -> bool {
    test_rule(&rule.name)
        || production_rule(&rule.name) && truthy(rule.attributes.get(TEST_ONLY_ATTRIBUTE))
}

fn truthy(expression: Option<&Expression>) -> bool {
    matches!(expression, Some(Expression::Name(value)) if value == TRUE_LITERAL)
}

fn string(expression: Option<&Expression>) -> Option<&str> {
    match expression {
        Some(Expression::String(value)) => Some(value),
        _ => None,
    }
}
