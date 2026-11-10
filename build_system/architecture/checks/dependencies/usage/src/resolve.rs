use std::collections::BTreeSet;
use yolo_architecture_target_ownership::{Document, Expression, Repository};

pub(super) fn dependencies(
    repository: &Repository,
    document: &Document,
    line: usize,
    expression: &Expression,
) -> Option<Vec<String>> {
    resolve(repository, document, line, expression, &mut BTreeSet::new())
}

fn resolve(
    repository: &Repository,
    document: &Document,
    line: usize,
    expression: &Expression,
    visiting: &mut BTreeSet<String>,
) -> Option<Vec<String>> {
    match expression {
        Expression::String(label) => Some(vec![label.clone()]),
        Expression::Add(values) | Expression::List(values) => {
            values.iter().try_fold(Vec::new(), |mut labels, value| {
                labels.extend(resolve(repository, document, line, value, visiting)?);
                Some(labels)
            })
        }
        Expression::Name(name) => resolve_name(repository, document, line, name, visiting),
        Expression::Call(_) | Expression::Unknown => None,
    }
}

fn resolve_name(
    repository: &Repository,
    document: &Document,
    line: usize,
    name: &str,
    visiting: &mut BTreeSet<String>,
) -> Option<Vec<String>> {
    let key = format!("{}#{name}", document.path);
    if !visiting.insert(key.clone()) {
        return None;
    }
    let result = match document
        .assignments
        .get(name)
        .and_then(|values| values.iter().rev().find(|value| value.line <= line))
    {
        Some(assignment) => resolve(
            repository,
            document,
            assignment.line,
            &assignment.expression,
            visiting,
        ),
        None => imported(repository, document, name, visiting),
    };
    visiting.remove(&key);
    result
}

fn imported(
    repository: &Repository,
    document: &Document,
    name: &str,
    visiting: &mut BTreeSet<String>,
) -> Option<Vec<String>> {
    let origin = document.imports.get(name)?;
    let (path, symbol) = origin
        .split_once('#')
        .unwrap_or((origin.as_str(), name));
    let source = repository.documents().find(|source| source.path == path)?;
    resolve_name(repository, source, usize::MAX, symbol, visiting)
}
