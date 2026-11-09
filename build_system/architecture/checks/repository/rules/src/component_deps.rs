use std::collections::{BTreeMap, BTreeSet};
use yolo_architecture_foundation::{SourceFile, model::Violation, text::contains_identifier};
use yolo_architecture_syntax::starlark_lists::{Element, list_entries};
use yolo_architecture_syntax::targeted_tokens::{Kind, tokenize};

const COMPONENT_DEPS_FILE: &str = "build_system/bazel/rust/component_deps.bzl";
const TOPOLOGY_FILE: &str = "build_system/architecture/topology.bzl";
const PROVIDERS_FILE: &str = "build_system/bazel/rust/providers.bzl";
const PROVIDER_ROWS_PREFIX: &str = "build_system/bazel/rust/provider_rows/";
const PROVIDER_COMPONENTS_BLOCK: &str = "runtime_components = [";
const COMPONENT_DEPS_ASSIGNMENT: &str = "RUNTIME_COMPONENT_DEPS";
const FACADE_DIRECTORY: &str = "core/src/";
const ASSIGNMENT: &str = "=";
const POLICY_MESSAGE: &str =
    "runtime component dependency labels must be referenced by the core facade sources";
const MALFORMED_POLICY: &str = "runtime component dependency list must be a literal list";
const MISSING_NAME_POLICY: &str = "runtime component dependency name has no string assignment";

#[derive(Clone)]
enum Value {
    Label(String),
    Name(String),
}

pub fn inspect(files: &[SourceFile], violations: &mut Vec<Violation>) {
    let Some(deps) = files
        .iter()
        .find(|file| file.relative == COMPONENT_DEPS_FILE)
    else {
        return;
    };
    let entries = match list_entries(&deps.text, COMPONENT_DEPS_ASSIGNMENT) {
        Ok(entries) => entries,
        Err(error) => {
            violations.push(Violation::line(
                &deps.relative,
                error.line,
                MALFORMED_POLICY,
            ));
            return;
        }
    };
    let mut values = files
        .iter()
        .find(|file| file.relative == TOPOLOGY_FILE)
        .map(|file| assignments(&file.text))
        .unwrap_or_default();
    values.extend(assignments(&deps.text));
    let facade = files
        .iter()
        .filter(|file| file.relative.starts_with(FACADE_DIRECTORY))
        .map(|file| file.text.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    for (line, label) in provider_runtime_components(files) {
        let name = label.rsplit(':').next().unwrap_or(&label);
        if !contains_identifier(&facade, name) {
            violations.push(Violation::line(
                PROVIDERS_FILE,
                line,
                format!("{POLICY_MESSAGE}: {label}"),
            ));
        }
    }
    for entry in entries {
        let (label, missing_name) = match entry.element {
            Element::String(label) => (Some(label), None),
            Element::Name(name) => (resolve(&name, &values, &mut BTreeSet::new()), Some(name)),
        };
        let Some(label) = label else {
            violations.push(Violation::line(
                &deps.relative,
                entry.line,
                format!(
                    "{MISSING_NAME_POLICY}: {}",
                    missing_name.unwrap_or_default()
                ),
            ));
            continue;
        };
        let name = label.rsplit(':').next().unwrap_or(&label);
        if !contains_identifier(&facade, name) {
            violations.push(Violation::line(
                &deps.relative,
                entry.line,
                format!("{POLICY_MESSAGE}: {label}"),
            ));
        }
    }
}

fn assignments(text: &str) -> BTreeMap<String, Value> {
    let tokens = tokenize(text);
    let mut values = BTreeMap::new();
    for index in 0..tokens.len().saturating_sub(2) {
        let Kind::Identifier(name) = &tokens[index].kind else {
            continue;
        };
        if !is_constant(name) || !tokens[index + 1].symbol(ASSIGNMENT) {
            continue;
        }
        match &tokens[index + 2].kind {
            Kind::String(label) => {
                values.insert(name.clone(), Value::Label(label.clone()));
            }
            Kind::Identifier(next) => {
                values.insert(name.clone(), Value::Name(next.clone()));
            }
            _ => {}
        }
    }
    values
}

fn resolve(name: &str, values: &BTreeMap<String, Value>, visiting: &mut BTreeSet<String>) -> Option<String> {
    if !visiting.insert(name.to_owned()) {
        return None;
    }
    let result = match values.get(name)? {
        Value::Label(label) => Some(label.clone()),
        Value::Name(next) => resolve(next, values, visiting),
    };
    visiting.remove(name);
    result
}

fn is_constant(name: &str) -> bool {
    name.chars()
        .all(|character| character.is_ascii_uppercase() || character == '_')
}

/// Reads the provider runtime component labels from the single provider table.
/// The table itself owns the list; the facade rule above still requires every
/// derived label to be re-exported by the core facade.
fn provider_runtime_components(files: &[SourceFile]) -> Vec<(usize, String)> {
    let mut entries = Vec::new();
    for providers in files.iter().filter(|file| {
        file.relative == PROVIDERS_FILE || file.relative.starts_with(PROVIDER_ROWS_PREFIX)
    }) {
        let mut inside = false;
        for (index, line) in providers.text.lines().enumerate() {
            let trimmed = line.trim();
            if !inside {
                inside = trimmed.starts_with(PROVIDER_COMPONENTS_BLOCK);
                continue;
            }
            if trimmed.starts_with(']') {
                inside = false;
                continue;
            }
            if let Some(label) = first_label(trimmed) {
                entries.push((index + 1, label));
            }
        }
    }
    entries
}

fn first_label(line: &str) -> Option<String> {
    let start = line.find('"')? + 1;
    let rest = &line[start..];
    let end = rest.find('"')?;
    let label = &rest[..end];
    label.starts_with("//core/").then(|| label.to_owned())
}
