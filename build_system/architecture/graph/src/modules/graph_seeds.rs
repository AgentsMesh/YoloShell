use super::ModuleIndex;
use super::files;
use super::pending_states::State;
use crate::crates::Catalog as CrateCatalog;
use std::collections::{BTreeSet, VecDeque};
use yolo_architecture_foundation::SourceFile;
use yolo_architecture_source_policy::source_policy;
use yolo_architecture_syntax::rust_source::RustSource;

pub(super) fn seeds(
    sources: &[(&SourceFile, RustSource)],
    crates: &CrateCatalog,
    available: &BTreeSet<String>,
    modules: &mut ModuleIndex,
) -> VecDeque<State> {
    let explicit = crates
        .roots
        .iter()
        .filter_map(|root| {
            if !available.contains(&root.source) {
                modules.mark_uncertain(&root.source);
                return None;
            }
            Some(state(
                &root.source,
                vec![root.source.clone()],
                Some(root.source.clone()),
                root.product,
            ))
        })
        .collect::<VecDeque<_>>();
    if !crates.roots.is_empty() {
        return explicit;
    }
    sources
        .iter()
        .filter(|(file, _)| !source_policy::is_rust_test_source(&file.relative))
        .map(|(file, _)| {
            let crate_root =
                files::conventional_crate_root(&file.relative).then(|| file.relative.clone());
            state(
                &file.relative,
                ModuleIndex::inferred(&file.relative),
                crate_root,
                true,
            )
        })
        .collect()
}

fn state(file: &str, path: Vec<String>, crate_root: Option<String>, product: bool) -> State {
    State {
        file: file.to_owned(),
        path,
        lineage: std::iter::once(file.to_owned()).collect(),
        crate_root,
        module_search_directory: None,
        product,
    }
}
