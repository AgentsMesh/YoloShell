use crate::constants::{
    self as contract, RUST_BINARY_RULE, RUST_LIBRARY_RULE, RUST_STATIC_LIBRARY_RULE,
    RUST_TEST_RULE, TEST_ONLY_ATTRIBUTE, TRUE_LITERAL,
};
use yolo_architecture_target_ownership::{Document, Expression, Repository, RuleCall};
use std::collections::BTreeSet;
use yolo_architecture_foundation::model::Violation;

pub fn inspect(document: &Document, rule: &RuleCall, violations: &mut Vec<Violation>) {
    let dependencies = rule
        .attributes
        .get(contract::DEPENDENCIES_ATTRIBUTE)
        .and_then(|expression| resolve(document, rule.line, expression, &mut BTreeSet::new()));
    if dependencies.as_ref().is_none_or(|labels| {
        labels
            .iter()
            .any(|label| !contract::ALLOWED_DEPENDENCIES.contains(&label.as_str()))
    }) {
        violations.push(Violation::line(
            &document.path,
            rule.line,
            contract::DEPENDENCY_POLICY,
        ));
    }
}

pub fn resolve(
    document: &Document,
    line: usize,
    expression: &Expression,
    visiting: &mut BTreeSet<String>,
) -> Option<Vec<String>> {
    match expression {
        Expression::String(label) => Some(vec![label.clone()]),
        Expression::Add(values) | Expression::List(values) => {
            values.iter().try_fold(Vec::new(), |mut labels, value| {
                labels.extend(resolve(document, line, value, visiting)?);
                Some(labels)
            })
        }
        Expression::Name(name) => {
            if !visiting.insert(name.clone()) {
                return None;
            }
            let assignments = document.assignments.get(name)?;
            let [assignment] = assignments.as_slice() else {
                return None;
            };
            if assignment.line > line {
                return None;
            }
            let labels = resolve(document, assignment.line, &assignment.expression, visiting);
            visiting.remove(name);
            labels
        }
        Expression::Call(_) | Expression::Unknown => None,
    }
}

pub fn macro_is_reviewed(
    repository: &Repository,
    document: &Document,
    visited: &mut BTreeSet<String>,
) -> bool {
    if document.rules_unresolved {
        return false;
    }
    if !visited.insert(document.path.clone()) {
        return true;
    }
    document.calls.iter().all(|call| {
        if call.definition { return true; }
        if let Some(origin) = document.imports.get(&call.name) {
            let (path, symbol) = origin.split_once('#').unwrap_or((origin.as_str(), call.name.as_str()));
            return contract::REVIEWED_MACROS.contains(&(path, symbol))
                && repository.document(path).is_some_and(|source| macro_is_reviewed(repository, source, visited));
        }
        if !call.qualified && let Some(assignments) = document.assignments.get(&call.name) {
            return matches!(assignments.as_slice(), [assignment]
                if matches!(&assignment.expression, Expression::Call(value) if value.name == "rule"));
        }
        if call.canonical == RUST_BINARY_RULE {
            return document.rules.iter().any(|rule| rule.line == call.line && rule.name == RUST_BINARY_RULE
                && matches!(rule.attributes.get(TEST_ONLY_ATTRIBUTE), Some(Expression::Name(value)) if value == TRUE_LITERAL));
        }
        if call.qualified { return contract::MACRO_METHODS.contains(&call.name.as_str())
            || capnp_method(&document.path, &call.name)
            || call.depset_receiver && call.name == "to_list"; }
        matches!(call.canonical.as_str(), RUST_LIBRARY_RULE | RUST_STATIC_LIBRARY_RULE | RUST_TEST_RULE)
            || contract::MACRO_BUILTINS.contains(&call.name.as_str())
            || document.path == contract::CAPNP_GENERATION && contract::CAPNP_BUILTINS.contains(&call.name.as_str())
            || document.calls.iter().any(|definition| definition.definition && definition.name == call.name)
    })
}

// Extra action-building primitives belong only to the reviewed schema generator.
// Imported symbols and nested aliases still pass the recursive callable review.
fn capnp_method(path: &str, name: &str) -> bool {
    path == contract::CAPNP_DEFINITIONS && name == "filegroup"
        || path == contract::CAPNP_GENERATION && contract::CAPNP_METHODS.contains(&name)
}

pub fn inspect_launch_contract(
    document: &Document,
    rule: &RuleCall,
    violations: &mut Vec<Violation>,
) {
    if rule
        .attributes
        .get(contract::DEPENDENCIES_ATTRIBUTE)
        .is_some_and(|value| {
            resolve(document, rule.line, value, &mut BTreeSet::new())
                .is_none_or(|labels| !labels.is_empty())
        })
    {
        violations.push(Violation::line(
            &document.path,
            rule.line,
            contract::PURE_LAUNCH_POLICY,
        ));
    }
}

pub(crate) mod plugin {
    use crate::constants::*;
    use super::resolve;
    use yolo_architecture_target_ownership::{Expression, Repository};
    use std::collections::BTreeSet;
    use yolo_architecture_foundation::model::Violation;

    const CORE_COMPONENTS: &str = "core/crates/";
    const COMPOSITION: &str = "core/crates/application/yolo";
    const CLIENT_LABELS: &[&str] = &["//clients/"];
    const UI_LIBRARIES: &[&str] = &["@crates//:ratatui", "@crates//:crossterm"];
    pub(crate) const MESSAGE: &str =
        "core components cannot depend on client implementations or UI libraries";

    pub fn inspect(repository: &Repository, violations: &mut Vec<Violation>) {
        for document in repository.documents().filter(|document| {
            document.package.starts_with(CORE_COMPONENTS) && document.package != COMPOSITION
        }) {
            for rule in &document.rules {
                if !matches!(
                    rule.name.as_str(),
                    RUST_LIBRARY_RULE | RUST_STATIC_LIBRARY_RULE
                ) || matches!(rule.attributes.get(TEST_ONLY_ATTRIBUTE),
                        Some(Expression::Name(value)) if value == TRUE_LITERAL)
                {
                    continue;
                }
                let Some(deps) = rule.attributes.get("deps") else {
                    continue;
                };
                let Some(labels) = resolve(document, rule.line, deps, &mut BTreeSet::new()) else {
                    continue;
                };
                for label in labels.iter().filter(|label| {
                    CLIENT_LABELS
                        .iter()
                        .any(|prefix| label.starts_with(prefix))
                        || UI_LIBRARIES.contains(&label.as_str())
                }) {
                    violations.push(Violation::line(
                        &document.path,
                        rule.line,
                        format!("{MESSAGE}: {label}"),
                    ));
                }
            }
        }
    }
}
