use crate::Repository;
use crate::constants::{
    BUILD_SYSTEM_PREFIX, CORE_PREFIX, CROSS_PACKAGE_TEST_OWNERSHIP_MESSAGE,
    INTEGRATION_TEST_DIRECTORY, INTEGRATION_TESTS_PREFIX, MISSING_TEST_OWNER_MESSAGE,
    MIXED_TEST_CLASS_MESSAGE, UNIT_TEST_DIRECTORY, PLUGINS_PREFIX,
};
use crate::model::Target;
use std::collections::BTreeSet;
use std::path::{Component, Path};
use yolo_architecture_foundation::model::Violation;

pub fn inspect(repository: &Repository, targets: &[Target], violations: &mut Vec<Violation>) {
    inspect_owners(repository, targets, violations);
    for target in targets {
        inspect_package_boundary(repository, target, violations);
        inspect_test_class(target, violations);
    }
}

fn inspect_owners(repository: &Repository, targets: &[Target], violations: &mut Vec<Violation>) {
    let owned = targets
        .iter()
        .flat_map(|target| target.sources.iter().map(String::as_str))
        .collect::<BTreeSet<_>>();
    for source in repository.rust_sources().filter(|source| protected(source)) {
        if crate::paths::is_test_owned_path(source) && !owned.contains(source.as_str()) {
            violations.push(Violation::file(source, MISSING_TEST_OWNER_MESSAGE));
        }
    }
}

fn inspect_package_boundary(
    repository: &Repository,
    target: &Target,
    violations: &mut Vec<Violation>,
) {
    let crosses_package = target
        .sources
        .iter()
        .filter(|source| protected(source))
        .any(|source| {
            repository.source_package(source).as_deref() != Some(target.package.as_str())
        });
    if crosses_package {
        violations.push(Violation::line(
            &target.path,
            target.line,
            CROSS_PACKAGE_TEST_OWNERSHIP_MESSAGE,
        ));
    }
}

fn inspect_test_class(target: &Target, violations: &mut Vec<Violation>) {
    let relative = |source: &String| {
        Path::new(source)
            .strip_prefix(&target.package)
            .unwrap_or(Path::new(source))
            .to_path_buf()
    };
    let has_unit = target
        .sources
        .iter()
        .any(|source| contains_directory(&relative(source), UNIT_TEST_DIRECTORY));
    let has_integration = target
        .sources
        .iter()
        .any(|source| contains_directory(&relative(source), INTEGRATION_TEST_DIRECTORY));
    if has_unit && has_integration {
        violations.push(Violation::line(
            &target.path,
            target.line,
            MIXED_TEST_CLASS_MESSAGE,
        ));
    }
}

fn protected(path: &str) -> bool {
    path.starts_with(BUILD_SYSTEM_PREFIX)
        || path.starts_with(CORE_PREFIX)
        || path.starts_with(PLUGINS_PREFIX)
        || path.starts_with(INTEGRATION_TESTS_PREFIX)
}

fn contains_directory(path: &Path, directory: &str) -> bool {
    path.components()
        .any(|component| matches!(component, Component::Normal(name) if name == directory))
}
