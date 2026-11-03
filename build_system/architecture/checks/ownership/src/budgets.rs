use crate::Repository;
use crate::constants::{
    BUILD_SYSTEM_DIRECTORY, BUILD_SYSTEM_PREFIX, CORE_PACKAGE_LINE_BUDGET_MESSAGE,
    CORE_PACKAGE_SOURCE_BUDGET_MESSAGE, CORE_PREFIX, MAX_BUILD_SYSTEM_PRODUCTION_LINES_PER_PACKAGE,
    MAX_BUILD_SYSTEM_PRODUCTION_SOURCES_PER_PACKAGE,
    MAX_BUILD_SYSTEM_PRODUCTION_SOURCES_PER_TARGET, MAX_BUILD_SYSTEM_TEST_SOURCES_PER_TARGET,
    MAX_CORE_PRODUCTION_LINES_PER_PACKAGE, MAX_CORE_PRODUCTION_SOURCES_PER_PACKAGE,
    PACKAGE_LINE_BUDGET_MESSAGE, PACKAGE_SOURCE_BUDGET_MESSAGE, PLUGINS_PREFIX,
    RUST_SOURCE_EXTENSION, SOURCE_BUDGET_MESSAGE, TEST_SOURCE_BUDGET_MESSAGE,
    UNRESOLVED_SOURCE_EXPRESSION_MESSAGE,
};
use crate::model::Target;
use std::collections::{BTreeMap, BTreeSet};
use yolo_architecture_foundation::model::Violation;

pub fn inspect(
    repository: &Repository,
    production_sources: &BTreeSet<String>,
    targets: &[Target],
    test_targets: &[Target],
    violations: &mut Vec<Violation>,
) {
    inspect_targets(production_sources, targets, violations);
    inspect_packages(repository, production_sources, targets, violations);
    inspect_test_targets(production_sources, test_targets, violations);
}

fn inspect_test_targets(
    production_sources: &BTreeSet<String>,
    targets: &[Target],
    violations: &mut Vec<Violation>,
) {
    for target in targets
        .iter()
        .filter(|target| target.package.starts_with(BUILD_SYSTEM_DIRECTORY))
    {
        inspect_resolution(target, violations);
        let count = target
            .sources
            .iter()
            .filter(|source| {
                !production_sources.contains(*source)
                    && source
                        .rsplit_once('.')
                        .is_some_and(|(_, extension)| extension == RUST_SOURCE_EXTENSION)
            })
            .count();
        let budget = MAX_BUILD_SYSTEM_TEST_SOURCES_PER_TARGET;
        if count > budget {
            violations.push(Violation::line(
                &target.path,
                target.line,
                format!("{TEST_SOURCE_BUDGET_MESSAGE}: {count} > {budget}"),
            ));
        }
    }
}

fn inspect_targets(
    production_sources: &BTreeSet<String>,
    targets: &[Target],
    violations: &mut Vec<Violation>,
) {
    for target in targets
        .iter()
        .filter(|target| target.package.starts_with(BUILD_SYSTEM_DIRECTORY))
    {
        inspect_resolution(target, violations);
        let count = target
            .sources
            .iter()
            .filter(|source| {
                source.starts_with(BUILD_SYSTEM_PREFIX) && production_sources.contains(*source)
            })
            .count();
        let budget = MAX_BUILD_SYSTEM_PRODUCTION_SOURCES_PER_TARGET;
        if count > budget {
            violations.push(Violation::line(
                &target.path,
                target.line,
                format!("{SOURCE_BUDGET_MESSAGE}: {count} > {budget}"),
            ));
        }
    }
}

fn inspect_packages(
    repository: &Repository,
    production_sources: &BTreeSet<String>,
    targets: &[Target],
    violations: &mut Vec<Violation>,
) {
    let mut packages = BTreeMap::<String, (usize, usize)>::new();
    for source in production_sources.iter().filter(|source| {
        let path = source.as_str();
        path.starts_with(BUILD_SYSTEM_PREFIX)
            || path.starts_with(CORE_PREFIX)
            || path.starts_with(PLUGINS_PREFIX)
    }) {
        if let Some(package) = repository.source_package(source) {
            let entry = packages.entry(package).or_default();
            entry.0 += 1;
            entry.1 += repository.rust_source_lines(source).unwrap_or(0);
        }
    }
    for (package, (count, line_count)) in packages {
        let Some(target) = targets.iter().find(|target| target.package == package) else {
            continue;
        };
        let recorded = crate::constants::PACKAGE_BASELINE
            .iter()
            .find(|(entry, _, _)| *entry == package)
            .map(|(_, sources, lines)| (*sources, *lines));
        let build_system = package.starts_with(BUILD_SYSTEM_DIRECTORY);
        let budget = if build_system {
            (
                MAX_BUILD_SYSTEM_PRODUCTION_SOURCES_PER_PACKAGE,
                MAX_BUILD_SYSTEM_PRODUCTION_LINES_PER_PACKAGE,
            )
        } else {
            (
                MAX_CORE_PRODUCTION_SOURCES_PER_PACKAGE,
                MAX_CORE_PRODUCTION_LINES_PER_PACKAGE,
            )
        };
        let message = if build_system {
            (PACKAGE_SOURCE_BUDGET_MESSAGE, PACKAGE_LINE_BUDGET_MESSAGE)
        } else {
            (
                CORE_PACKAGE_SOURCE_BUDGET_MESSAGE,
                CORE_PACKAGE_LINE_BUDGET_MESSAGE,
            )
        };
        for (value, budget, recorded, message) in [
            (count, budget.0, recorded.map(|entry| entry.0), message.0),
            (line_count, budget.1, recorded.map(|entry| entry.1), message.1),
        ] {
            let limit = recorded.map_or(budget, |recorded| recorded.max(budget));
            if value > limit {
                violations.push(Violation::file(
                    &target.path,
                    format!("{message}: {value} > {limit}"),
                ));
            }
        }
    }
}

fn inspect_resolution(target: &Target, violations: &mut Vec<Violation>) {
    if target.sources_unresolved {
        violations.push(Violation::line(
            &target.path,
            target.line,
            UNRESOLVED_SOURCE_EXPRESSION_MESSAGE,
        ));
    }
}
