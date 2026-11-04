use crate::constants::{
    BUILD_SYSTEM_PREFIX, CROSS_PACKAGE_AGGREGATION_MESSAGE, DUPLICATE_OWNER_MESSAGE,
    FACADE_OWNERSHIP_MESSAGE, MISSING_OWNER_MESSAGE, PRODUCTION_TEST_SOURCE_MESSAGE,
    SOURCE_GROUP_SUFFIX,
};
use crate::model::Target;
use crate::{Repository, production, targets, test_ownership};
use std::collections::{BTreeMap, BTreeSet};
use yolo_architecture_foundation::{SourceFile, model::Violation};

pub fn inspect(files: &[SourceFile], violations: &mut Vec<Violation>) {
    let repository = Repository::new(files);
    let targets = targets::collect(&repository);
    let test_targets = targets::collect_tests(&repository);
    let production_sources = production::sources(&repository, &targets);
    crate::layout::inspect(&repository, &production_sources, violations);
    inspect_owners(&production_sources, &targets, violations);
    test_ownership::inspect(&repository, &test_targets, violations);
    crate::reachability::inspect(files, &repository, &test_targets, violations);
    inspect_groups(&repository, &production_sources, violations);
    inspect_targets(&repository, &production_sources, &targets, violations);
    crate::budgets::inspect(
        &repository,
        &production_sources,
        &targets,
        &test_targets,
        violations,
    );
    crate::path_rules::inspect(files, &repository, &production_sources, violations);
}
fn inspect_owners(
    production_sources: &BTreeSet<String>,
    targets: &[Target],
    violations: &mut Vec<Violation>,
) {
    let mut owners: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for target in targets {
        for source in &target.sources {
            if production_sources.contains(source) {
                owners.entry(source).or_default().push(&target.id);
            }
        }
    }
    for source in production_sources {
        let source_owners = owners.get(source.as_str()).cloned().unwrap_or_default();
        if source_owners.is_empty() {
            violations.push(Violation::file(source, MISSING_OWNER_MESSAGE));
        } else if source_owners.len() > 1 {
            violations.push(Violation::file(
                source,
                format!("{DUPLICATE_OWNER_MESSAGE}: {}", source_owners.join(", ")),
            ));
        }
    }
}
fn inspect_groups(
    repository: &Repository,
    production_sources: &BTreeSet<String>,
    violations: &mut Vec<Violation>,
) {
    for group in repository.groups() {
        if !group.name.ends_with(SOURCE_GROUP_SUFFIX) {
            continue;
        }
        let evaluated = repository.evaluate(
            &group.document,
            &group.package,
            group.line,
            &group.expression,
        );
        if evaluated.paths.iter().any(|source| {
            production_sources.contains(source)
                && repository.source_package(source).as_deref() != Some(&group.package)
        }) {
            violations.push(Violation::line(
                &group.document,
                group.line,
                CROSS_PACKAGE_AGGREGATION_MESSAGE,
            ));
        }
    }
}
fn inspect_targets(
    repository: &Repository,
    production_sources: &BTreeSet<String>,
    targets: &[Target],
    violations: &mut Vec<Violation>,
) {
    for target in targets {
        if target.sources.iter().any(|source| {
            source.starts_with(BUILD_SYSTEM_PREFIX) && crate::paths::is_test_owned_path(source)
        }) {
            violations.push(Violation::line(
                &target.path,
                target.line,
                PRODUCTION_TEST_SOURCE_MESSAGE,
            ));
        }
        let target_sources = target
            .sources
            .iter()
            .filter(|source| production_sources.contains(*source))
            .collect::<Vec<_>>();
        let crosses_package = target_sources
            .iter()
            .any(|source| repository.source_package(source).as_deref() != Some(&target.package));
        if crosses_package {
            violations.push(Violation::line(
                &target.path,
                target.line,
                FACADE_OWNERSHIP_MESSAGE,
            ));
        }
        if target.source_labels.iter().any(|label| {
            repository
                .group(label)
                .is_some_and(|group| group.package != target.package)
        }) {
            violations.push(Violation::line(
                &target.path,
                target.line,
                CROSS_PACKAGE_AGGREGATION_MESSAGE,
            ));
        }
    }
}
