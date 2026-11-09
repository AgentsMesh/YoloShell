use crate::constants::{BUILD_SYSTEM_PREFIX, CLIENTS_PREFIX, CORE_PREFIX};
use crate::{Repository, model::Target, paths, targets};
use std::collections::BTreeSet;

pub fn sources(repository: &Repository, production_targets: &[Target]) -> BTreeSet<String> {
    let production_owned = production_targets
        .iter()
        .flat_map(|target| &target.sources)
        .collect::<BTreeSet<_>>();
    // A testonly library/binary may have ordinary src/ files. Unit-test rules
    // recompiling product sources do not establish this fixture-only ownership.
    let helpers = targets::collect_test_helpers(repository);
    let helper_owned = helpers
        .iter()
        .flat_map(|target| &target.sources)
        .collect::<BTreeSet<_>>();
    repository
        .rust_sources()
        .filter(|path| is_production_path(path))
        .filter(|path| production_owned.contains(path) || !helper_owned.contains(path))
        .cloned()
        .collect()
}

fn is_production_path(path: &str) -> bool {
    if paths::is_test_path(path) {
        return false;
    }
    if path.starts_with(CORE_PREFIX) || path.starts_with(CLIENTS_PREFIX) {
        return true;
    }
    path.starts_with(BUILD_SYSTEM_PREFIX)
}
