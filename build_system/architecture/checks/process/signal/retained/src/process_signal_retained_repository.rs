use crate::process_signal_retained_macros;
use crate::process_signal_retained_unicode;
use crate::process_signal_retained_visibility;
use yolo_architecture_foundation::SourceFile;
use yolo_architecture_foundation::model::Violation;
use yolo_architecture_policy_contracts::process_signal_retained as retained;
use yolo_architecture_process_signal_foundation::process_signal_retained_operations;
use yolo_architecture_build_graph::crates;
use yolo_architecture_build_graph::modules::{self, ModuleIndex};
use yolo_architecture_process_signal_path_analysis::deref;
use yolo_architecture_process_signal_path_analysis::paths::{self, Catalog};
use yolo_architecture_process_signal_path_analysis::types;
use yolo_architecture_syntax::rust_source::RustSource;

const RUST_SOURCE_SUFFIX: &str = ".rs";
const MINIMUM_CATALOG_BUDGET: usize = 1;
const MAXIMUM_CATALOG_RESOLUTION_PASSES: usize = 4_096;

pub fn inspect(files: &[SourceFile], violations: &mut Vec<Violation>) {
    let graph_sources = files
        .iter()
        .filter(|file| rust_graph_source(file))
        .map(|file| (file, RustSource::parse(&file.text)))
        .collect::<Vec<_>>();
    let crates = crates::discover(files);
    for relative in &crates.unresolved {
        resolution_violation(relative, violations);
    }
    let modules = modules::discover(&graph_sources, &crates);
    let sources = files
        .iter()
        .filter(|file| rust_graph_source(file) && modules.is_product(&file.relative))
        .map(|file| (file, RustSource::parse(&file.text)))
        .collect::<Vec<_>>();
    let facts = sources
        .iter()
        .map(|(file, source)| paths::prepare(&file.relative, &source.tokens, &modules))
        .collect::<Vec<_>>();
    for relative in modules.uncertain_files() {
        resolution_violation(relative, violations);
    }
    let (exported, catalog_uncertain) = exported_catalog(&sources, &facts, &modules);
    for relative in &catalog_uncertain {
        resolution_violation(relative, violations);
    }
    for ((file, source), facts) in sources.into_iter().zip(&facts) {
        let resolution =
            paths::resolve_prepared(&file.relative, &source.tokens, &exported, &modules, facts);
        let retained = types::type_names_with(&source.tokens, &resolution);
        if resolution.uncertain {
            resolution_violation(&file.relative, violations);
        }
        process_signal_retained_unicode::inspect(file, &source, violations);
        crate::process_signal_retained_api::inspect(file, &source, violations);
        process_signal_retained_visibility::inspect_with_modules(
            file, &source, &retained, &modules, violations,
        );
        process_signal_retained_macros::inspect(file, &source, &retained, violations);
        deref::inspect(file, &source, &retained, violations);
        process_signal_retained_operations::inspect_with_resolution(
            file,
            &source,
            &resolution,
            violations,
        );
    }
}

fn exported_catalog(
    sources: &[(&SourceFile, RustSource)],
    facts: &[paths::ResolutionFacts],
    modules: &ModuleIndex,
) -> (Catalog, std::collections::BTreeSet<String>) {
    let mut exported = Catalog::default();
    let mut uncertain = std::collections::BTreeSet::new();
    let mut remaining = sources
        .iter()
        .map(|(_, source)| source.tokens.len())
        .sum::<usize>()
        .max(MINIMUM_CATALOG_BUDGET)
        .saturating_mul(modules.path_count().max(MINIMUM_CATALOG_BUDGET))
        .min(MAXIMUM_CATALOG_RESOLUTION_PASSES);
    loop {
        let mut additions = Catalog::default();
        for ((file, source), facts) in sources.iter().zip(facts) {
            let resolution =
                paths::resolve_prepared(&file.relative, &source.tokens, &exported, modules, facts);
            let retained = types::type_names_with(&source.tokens, &resolution);
            if resolution.uncertain {
                uncertain.insert(file.relative.clone());
            }
            for (index, name, canonical) in
                process_signal_retained_visibility::declared_types(source, &retained)
            {
                if !modules.module_item(&file.relative, index) {
                    continue;
                }
                for owner in modules.paths_at(&file.relative, index) {
                    let visible_from = if modules.uses_declared_crate_roots() {
                        paths::visibility::scope(&source.tokens, index, &owner)
                    } else {
                        owner.first().into_iter().cloned().collect()
                    };
                    additions.insert(owner, &name, &visible_from, canonical);
                }
            }
            for binding in &resolution.type_bindings {
                for mut destination in modules.paths_at(&file.relative, binding.index) {
                    destination.push(binding.name.clone());
                    additions.alias_type(
                        &destination,
                        &binding.target,
                        &binding.visible_from,
                        binding.canonical,
                    );
                }
            }
            for alias in &resolution.module_bindings {
                for mut destination in modules.paths_at(&file.relative, alias.index) {
                    destination.push(alias.name.clone());
                    additions.alias_module(&destination, &alias.target, &alias.visible_from);
                }
            }
        }
        if !exported.merge(additions) {
            return (exported, uncertain);
        }
        if remaining == 0 {
            uncertain.extend(sources.iter().map(|(file, _)| file.relative.clone()));
            return (exported, uncertain);
        }
        remaining = remaining.saturating_sub(MINIMUM_CATALOG_BUDGET);
    }
}

fn resolution_violation(relative: &str, violations: &mut Vec<Violation>) {
    violations.push(Violation::file(
        relative,
        retained::RESOLUTION_INDETERMINATE_MESSAGE,
    ));
}

fn rust_graph_source(file: &SourceFile) -> bool {
    file.relative.ends_with(RUST_SOURCE_SUFFIX)
}
