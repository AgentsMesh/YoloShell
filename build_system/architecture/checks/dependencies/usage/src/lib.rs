mod baseline;
mod naming;
mod resolve;

use std::collections::BTreeSet;
use yolo_architecture_foundation::{SourceFile, model::Violation};
use yolo_architecture_policy_contracts::build_constants::{
    RUST_BINARY_RULE, RUST_LIBRARY_RULE, RUST_STATIC_LIBRARY_RULE, TESTONLY_ATTRIBUTE,
};
use yolo_architecture_target_entrypoint::{COMPOSITION_PACKAGE, DEPENDENCIES_ATTRIBUTE};
use yolo_architecture_target_ownership::{
    Expression, OWNERSHIP_POLICY_BUILD_FILE, Repository, RuleCall, SOURCES_ATTRIBUTE,
    TRUE_LITERAL, is_build_file,
};

use baseline::{STALE_POLICY, sites};
use naming::{crate_names, dependency_crate_name, referenced, target_name};

const POLICY_MESSAGE: &str = "aggregate dependency is not referenced by the target sources";
const COMPOSITION_PACKAGES: &[&str] = &[COMPOSITION_PACKAGE, "core"];

#[cfg(test)]
#[path = "../unit_tests/dependency_usage_tests.rs"]
mod tests;

pub fn inspect(files: &[SourceFile], violations: &mut Vec<Violation>) {
    if !files
        .iter()
        .any(|file| file.relative == OWNERSHIP_POLICY_BUILD_FILE)
    {
        return;
    }
    let repository = Repository::new(files);
    let texts = files
        .iter()
        .map(|file| (file.relative.as_str(), file.text.as_str()))
        .collect::<std::collections::BTreeMap<_, _>>();
    let baseline = sites(files, violations);
    let crates = crate_names(&repository);
    let mut recorded = BTreeSet::new();
    for document in repository
        .documents()
        .filter(|document| is_build_file(&document.path))
    {
        if COMPOSITION_PACKAGES.contains(&document.package.as_str()) {
            continue;
        }
        for rule in &document.rules {
            if !production_rule(&rule.name) || test_only(rule) {
                continue;
            }
            let target = target_name(rule).to_owned();
            if generated_sources(document, rule) {
                preserve_baseline(&baseline, &mut recorded, &document.path, &target);
                continue;
            }
            let Some(dependencies) = rule.attributes.get(DEPENDENCIES_ATTRIBUTE) else {
                continue;
            };
            if !list_driven(dependencies) {
                continue;
            }
            let Some(labels) = resolve::dependencies(&repository, document, rule.line, dependencies)
            else {
                preserve_baseline(&baseline, &mut recorded, &document.path, &target);
                continue;
            };
            let Some(sources) = rule.attributes.get(SOURCES_ATTRIBUTE) else {
                preserve_baseline(&baseline, &mut recorded, &document.path, &target);
                continue;
            };
            let evaluated =
                repository.evaluate(&document.path, &document.package, rule.line, sources);
            if evaluated.unresolved {
                preserve_baseline(&baseline, &mut recorded, &document.path, &target);
                continue;
            }
            let Some(text) = evaluated
                .paths
                .iter()
                .map(|path| texts.get(path.as_str()).copied())
                .collect::<Option<Vec<_>>>()
            else {
                preserve_baseline(&baseline, &mut recorded, &document.path, &target);
                continue;
            };
            let text = text.join("\n");
            for dependency in labels {
                let name = dependency_crate_name(document, dependency.as_str(), &crates);
                if referenced(&text, &name) {
                    continue;
                }
                let site = (document.path.clone(), target.clone(), dependency.clone());
                if baseline.contains(&site) {
                    recorded.insert(site);
                } else {
                    violations.push(Violation::line(
                        &document.path,
                        rule.line,
                        format!("{POLICY_MESSAGE}: {dependency}"),
                    ));
                }
            }
        }
    }
    for (path, target, dependency) in baseline.difference(&recorded) {
        violations.push(Violation::file(
            path,
            format!("{STALE_POLICY}: {target}: {dependency}"),
        ));
    }
}

fn generated_sources(document: &yolo_architecture_target_ownership::Document, rule: &RuleCall) -> bool {
    document.calls.iter().any(|call| {
        call.line == rule.line
            && call.name != rule.name
            && document.imports.get(&call.name).is_some_and(|origin| {
                let symbol = origin
                    .rsplit_once('#')
                    .map_or(origin.as_str(), |(_, symbol)| symbol);
                !matches!(
                    symbol,
                    RUST_LIBRARY_RULE | RUST_STATIC_LIBRARY_RULE | RUST_BINARY_RULE
                )
            })
    })
}

fn preserve_baseline(
    baseline: &BTreeSet<(String, String, String)>,
    recorded: &mut BTreeSet<(String, String, String)>,
    path: &str,
    target: &str,
) {
    recorded.extend(
        baseline
            .iter()
            .filter(|(site_path, site_target, _)| site_path == path && site_target == target)
            .cloned(),
    );
}

fn production_rule(name: &str) -> bool {
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

fn list_driven(expression: &Expression) -> bool {
    match expression {
        Expression::String(_) => false,
        Expression::List(values) => values.iter().any(list_driven),
        Expression::Add(values) => values.iter().any(list_driven),
        Expression::Name(_) | Expression::Call(_) | Expression::Unknown => true,
    }
}
