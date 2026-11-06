use std::collections::BTreeSet;
use yolo_architecture_foundation::model::Violation;
use yolo_architecture_policy_contracts::build_constants::{
    NAME_ATTRIBUTE, RUST_BINARY_RULE, RUST_LIBRARY_RULE, RUST_STATIC_LIBRARY_RULE,
};
use yolo_architecture_target_ownership::{
    Expression, Repository, RuleCall, TRUE_LITERAL, is_build_file,
};

use crate::roles::{Classification, HUB_LABEL_PREFIX};

const HUB_PACKAGE: &str = "core/crates/hub";
const COVERAGE_POLICY: &str = "every production Hub target must appear in a role list";
const PHANTOM_POLICY: &str = "Hub role label has no declared target";
const STALE_POLICY: &str = "stale Hub classification: classified package has no BUILD file";
const NO_PRODUCTION_POLICY: &str =
    "stale Hub classification: classified package has no production target";
const TESTONLY_ATTRIBUTE: &str = "testonly";
pub(crate) fn inspect(
    repository: &Repository,
    classification: &Classification,
    boundaries: &str,
    violations: &mut Vec<Violation>,
) {
    let mut declared = BTreeSet::new();
    let mut packages = BTreeSet::new();
    let mut production = BTreeSet::new();
    for document in repository
        .documents()
        .filter(|document| is_build_file(&document.path))
    {
        if !hub_package(&document.package) {
            continue;
        }
        packages.insert(document.package.as_str());
        for rule in &document.rules {
            let Some(name) = string(rule.attributes.get(NAME_ATTRIBUTE)) else {
                continue;
            };
            let label = format!("//{}:{}", document.package, name);
            declared.insert(label.clone());
            if production_rule(rule) {
                production.insert(document.package.as_str());
            }
            if production_rule(rule) && !classification.classified(&label) {
                violations.push(Violation::line(
                    &document.path,
                    rule.line,
                    format!("{COVERAGE_POLICY}: {label}"),
                ));
            }
        }
    }
    for label in classification
        .labels()
        .filter(|label| label.starts_with(HUB_LABEL_PREFIX))
    {
        if !declared.contains(label) {
            violations.push(Violation::file(
                boundaries,
                format!("{PHANTOM_POLICY}: {label}"),
            ));
        }
    }
    for package in classification.packages() {
        let package = package.trim_start_matches("//");
        if !packages.contains(package) {
            violations.push(Violation::file(
                boundaries,
                format!("{STALE_POLICY}: {package}"),
            ));
        } else if !production.contains(package) {
            violations.push(Violation::file(
                boundaries,
                format!("{NO_PRODUCTION_POLICY}: {package}"),
            ));
        }
    }
}

fn hub_package(package: &str) -> bool {
    package == HUB_PACKAGE || package.starts_with(&format!("{HUB_PACKAGE}/"))
}

fn production_rule(rule: &RuleCall) -> bool {
    !test_only(rule) && rust_rule(&rule.name)
}

fn rust_rule(name: &str) -> bool {
    matches!(
        name,
        RUST_LIBRARY_RULE | RUST_STATIC_LIBRARY_RULE | RUST_BINARY_RULE
    )
}

fn test_only(rule: &RuleCall) -> bool {
    matches!(
        rule.attributes.get(TESTONLY_ATTRIBUTE),
        Some(Expression::Name(value)) if value == TRUE_LITERAL
    )
}

fn string(expression: Option<&Expression>) -> Option<&str> {
    match expression {
        Some(Expression::String(value)) => Some(value),
        _ => None,
    }
}
