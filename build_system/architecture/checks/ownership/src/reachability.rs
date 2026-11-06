use crate::Repository;
use crate::constants::{
    INTEGRATION_TEST_DIRECTORY, UNCERTAIN_TEST_MODULE_MESSAGE, UNIT_TEST_DIRECTORY,
    UNREACHABLE_TEST_MODULE_MESSAGE, UNRESOLVED_TEST_ROOT_MESSAGE,
};
use crate::model::Target;
use crate::{reachability_config, reachability_tokens};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path};
use yolo_architecture_foundation::{SourceFile, model::Violation};
use yolo_architecture_build_graph::{
    crates::{Catalog, CrateRoot},
    modules,
};
use yolo_architecture_syntax::rust_source::RustSource;

const RUST_SUFFIX: &str = ".rs";

pub(super) fn inspect(
    files: &[SourceFile],
    repository: &Repository,
    targets: &[Target],
    violations: &mut Vec<Violation>,
) {
    let sources = files
        .iter()
        .filter(|file| file.relative.ends_with(RUST_SUFFIX))
        .map(|file| {
            (
                file.relative.as_str(),
                (file, RustSource::parse(&file.text)),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let mut candidates = BTreeSet::new();
    let mut reached = BTreeSet::new();
    let mut uncertain = BTreeSet::new();
    for target in targets {
        let Some(configuration) = reachability_config::for_target(repository, target) else {
            continue;
        };
        let target_tests = target
            .sources
            .iter()
            .filter(|source| test_source(target, source))
            .cloned()
            .collect::<BTreeSet<_>>();
        if target_tests.is_empty() {
            continue;
        }
        candidates.extend(target_tests.iter().cloned());
        let configuration = match configuration {
            Ok(configuration) => configuration,
            Err(()) => {
                violations.push(Violation::line(
                    &target.path,
                    target.line,
                    UNRESOLVED_TEST_ROOT_MESSAGE,
                ));
                uncertain.extend(target_tests);
                continue;
            }
        };
        let parsed = target
            .sources
            .iter()
            .filter_map(|path| {
                let (file, source) = sources.get(path.as_str())?;
                reachability_tokens::configure(&source.tokens, &configuration)
                    .map(|parsed| (*file, parsed))
            })
            .collect::<Vec<_>>();
        let catalog = Catalog {
            roots: BTreeSet::from([CrateRoot {
                id: target.id.clone(),
                source: configuration.root.clone(),
                product: true,
            }]),
            unresolved: BTreeSet::new(),
        };
        let index = modules::discover(&parsed, &catalog);
        if configuration.test {
            reached.extend(
                parsed
                    .iter()
                    .filter(|(file, _)| {
                        !index.paths_at(&file.relative, usize::default()).is_empty()
                    })
                    .map(|(file, _)| file.relative.clone()),
            );
        }
        if index.uncertain_files().next().is_some() {
            uncertain.extend(target_tests);
        }
    }
    for source in candidates.difference(&reached) {
        let message = if uncertain.contains(source) {
            UNCERTAIN_TEST_MODULE_MESSAGE
        } else {
            UNREACHABLE_TEST_MODULE_MESSAGE
        };
        violations.push(Violation::file(source, message));
    }
}

fn test_source(target: &Target, source: &str) -> bool {
    Path::new(source).strip_prefix(&target.package).ok().is_some_and(|relative| relative.components().any(|component| {
        matches!(component, Component::Normal(name) if name == UNIT_TEST_DIRECTORY || name == INTEGRATION_TEST_DIRECTORY)
    }))
}
