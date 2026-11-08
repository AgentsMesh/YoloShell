use std::collections::BTreeMap;
use yolo_architecture_policy_contracts::build_constants::{
    CRATE_NAME_ATTRIBUTE, NAME_ATTRIBUTE,
};
use yolo_architecture_target_ownership::{
    Document, Expression, Repository, RuleCall, is_build_file, label,
};

pub(super) fn target_name(rule: &RuleCall) -> &str {
    string(rule.attributes.get(NAME_ATTRIBUTE)).unwrap_or(&rule.name)
}

pub(super) fn crate_names(repository: &Repository) -> BTreeMap<String, String> {
    let mut names = BTreeMap::new();
    for document in repository
        .documents()
        .filter(|document| is_build_file(&document.path))
    {
        for rule in &document.rules {
            let Some(name) = string(rule.attributes.get(NAME_ATTRIBUTE)) else {
                continue;
            };
            let Some(crate_name) = string(rule.attributes.get(CRATE_NAME_ATTRIBUTE)) else {
                continue;
            };
            names.insert(label(&document.package, name), crate_name.to_owned());
        }
    }
    names
}

pub(super) fn dependency_crate_name(
    document: &Document,
    dependency: &str,
    crates: &BTreeMap<String, String>,
) -> String {
    let label = absolute_label(document, dependency);
    crates.get(&label).cloned().unwrap_or_else(|| {
        label
            .rsplit([':', '/'])
            .next()
            .unwrap_or(label.as_str())
            .replace('-', "_")
    })
}

fn absolute_label(document: &Document, dependency: &str) -> String {
    if let Some(name) = dependency.strip_prefix(':') {
        return label(&document.package, name);
    }
    if dependency.starts_with("//") && !dependency.contains(':') {
        if let Some(name) = dependency.rsplit('/').next() {
            return format!("{dependency}:{name}");
        }
    }
    dependency.to_owned()
}

pub(super) fn referenced(text: &str, name: &str) -> bool {
    text.match_indices(name).any(|(index, _)| {
        let before = text[..index].chars().next_back();
        let after = text[index + name.len()..].chars().next();
        before.is_none_or(|value| !identifier(value))
            && after.is_none_or(|value| !identifier(value))
    })
}

fn identifier(value: char) -> bool {
    value.is_alphanumeric() || value == '_'
}

fn string(expression: Option<&Expression>) -> Option<&str> {
    match expression? {
        Expression::String(value) => Some(value),
        _ => None,
    }
}
