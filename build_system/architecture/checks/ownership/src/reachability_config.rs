use crate::Repository;
use crate::constants::{
    CRATE_FEATURES_ATTRIBUTE, CRATE_ROOT_ATTRIBUTE, ISOLATED_RUST_TEST_RULE, NAME_ATTRIBUTE,
    RUST_TEST_RULE,
};
use crate::model::{Expression, RuleCall, Target};
use std::collections::BTreeSet;

pub(super) struct Configuration {
    pub root: String,
    pub test: bool,
    pub features: Option<BTreeSet<String>>,
}

pub(super) fn for_target(
    repository: &Repository,
    target: &Target,
) -> Option<Result<Configuration, ()>> {
    let document = repository.documents.get(&target.path)?;
    let rule = document
        .rules
        .iter()
        .find(|rule| matches_target(rule, target))?;
    let root = rule.attributes.get(CRATE_ROOT_ATTRIBUTE)?;
    let root = repository.evaluate(&target.path, &target.package, target.line, root);
    let [root_path] = root.paths.as_slice() else {
        return Some(Err(()));
    };
    if root.unresolved || !target.sources.contains(root_path) {
        return Some(Err(()));
    }
    Some(Ok(Configuration {
        root: root_path.clone(),
        test: matches!(rule.name.as_str(), RUST_TEST_RULE | ISOLATED_RUST_TEST_RULE),
        features: features(repository, target, rule),
    }))
}

fn matches_target(rule: &RuleCall, target: &Target) -> bool {
    let name = match rule.attributes.get(NAME_ATTRIBUTE) {
        Some(Expression::String(name)) => name,
        _ => &rule.name,
    };
    rule.line == target.line && format!("{}:{}:{name}", target.path, rule.line) == target.id
}

fn features(repository: &Repository, target: &Target, rule: &RuleCall) -> Option<BTreeSet<String>> {
    let Some(expression) = rule.attributes.get(CRATE_FEATURES_ATTRIBUTE) else {
        return Some(BTreeSet::new());
    };
    strings(
        repository,
        &target.path,
        target.line,
        expression,
        &mut BTreeSet::new(),
    )
}

fn strings(
    repository: &Repository,
    path: &str,
    line: usize,
    expression: &Expression,
    visited: &mut BTreeSet<String>,
) -> Option<BTreeSet<String>> {
    match expression {
        Expression::String(value) => Some(BTreeSet::from([value.clone()])),
        Expression::Add(values) | Expression::List(values) => {
            let mut result = BTreeSet::new();
            for value in values {
                result.extend(strings(repository, path, line, value, visited)?);
            }
            Some(result)
        }
        Expression::Name(name) => {
            let key = format!("{path}#{name}");
            if !visited.insert(key.clone()) {
                return None;
            }
            let result = binding(repository, path, line, name, visited);
            visited.remove(&key);
            result
        }
        Expression::Call(_) | Expression::Unknown => None,
    }
}

fn binding(
    repository: &Repository,
    path: &str,
    line: usize,
    name: &str,
    visited: &mut BTreeSet<String>,
) -> Option<BTreeSet<String>> {
    let document = repository.documents.get(path)?;
    if let Some(assignment) = document.assignments.get(name).and_then(|values| {
        values
            .iter()
            .rev()
            .find(|assignment| assignment.line <= line)
    }) {
        return strings(
            repository,
            path,
            assignment.line,
            &assignment.expression,
            visited,
        );
    }
    let import = document.imports.get(name)?;
    let (path, name) = import.split_once('#').unwrap_or((import, name));
    let expression = Expression::Name(name.to_owned());
    strings(repository, path, usize::MAX, &expression, visited)
}
