use std::collections::BTreeSet;
use std::path::PathBuf;
use yolo_architecture_foundation::SourceFile;
use yolo_architecture_policy_contracts::process_signal as contract;
use yolo_architecture_policy_contracts::process_signal_retained::BRIDGE_PATH_COMPONENT;
use yolo_architecture_build_graph::crates::{Catalog as CrateCatalog, CrateRoot};
use yolo_architecture_build_graph::modules;
use yolo_architecture_process_signal_path_analysis::paths::{self, Catalog as RetainedCatalog};
use yolo_architecture_process_signal_path_analysis::types;
use yolo_architecture_process_signal_policy::process_signal_retained_visibility;
use yolo_architecture_syntax::rust_source::RustSource;

const AUTHORITY_PATH: &str = "core/src/owner/child/authority.rs";
const CHILD_PATH: &str = "core/src/owner/child.rs";
const CLEAN: usize = 0;
pub(super) const CONSUMER_PATH: &str = "core/src/owner/child/nested/deeper.rs";
const CRATE_ID: &str = ROOT_PATH;
const EQUAL_VISIBILITY: &str = "pub(in crate::owner::child)";
pub(super) const EXTERNAL_TYPE: &str = "Aggregate";
const IMPORT: &str = "use crate::owner::child::authority::Aggregate;";
const NARROW_VISIBILITY: &str = "pub(super)";
const NESTED_PATH: &str = "core/src/owner/child/nested.rs";
const OWNER_PATH: &str = "core/src/owner.rs";
const REJECTED: usize = 1;
const ROOT_PATH: &str = "core/src/lib.rs";
const WIDE_VISIBILITY: &str = "pub(in crate::owner)";

#[test]
fn cross_file_aggregate_surfaces_preserve_the_source_audience() {
    for (visibility, expected) in [
        (WIDE_VISIBILITY, REJECTED),
        (EQUAL_VISIBILITY, CLEAN),
        (NARROW_VISIBILITY, CLEAN),
    ] {
        for surface in surfaces(visibility) {
            assert_count(&surface, expected);
        }
    }
}

#[test]
fn aggregate_wrappers_cannot_widen_the_source_audience() {
    for (visibility, expected) in [
        (WIDE_VISIBILITY, REJECTED),
        (EQUAL_VISIBILITY, CLEAN),
        (NARROW_VISIBILITY, CLEAN),
    ] {
        for wrapper in exposed_wrappers(visibility) {
            assert_count(&wrapper, expected);
        }
        for wrapper in private_wrappers(visibility) {
            assert_count(&wrapper, CLEAN);
        }
    }
}

#[test]
fn canonical_and_module_alias_globs_cannot_bypass_containment() {
    let direct = format!("{NARROW_VISIBILITY} use crate::owner::child::authority::*;");
    assert_catalog_count(&direct, true, false, REJECTED);
    let bridged = format!("{WIDE_VISIBILITY} use crate::owner::child::{BRIDGE_PATH_COMPONENT}::*;");
    assert_catalog_count(&bridged, false, true, REJECTED);
    let contained =
        format!("{NARROW_VISIBILITY} use crate::owner::child::{BRIDGE_PATH_COMPONENT}::*;");
    assert_catalog_count(&contained, false, true, CLEAN);
    assert_catalog_count(&contained, true, true, REJECTED);
}

fn surfaces(visibility: &str) -> Vec<String> {
    vec![
        format!("{IMPORT}\n{visibility} fn expose(_: {EXTERNAL_TYPE}) {{}}"),
        format!(
            "use crate::owner::child::authority::{EXTERNAL_TYPE} as Shared;\n\
             {visibility} fn expose(_: Shared) {{}}"
        ),
        format!("{visibility} fn expose(_: crate::owner::child::authority::{EXTERNAL_TYPE}) {{}}"),
        format!("{visibility} use crate::owner::child::authority::{EXTERNAL_TYPE} as Shared;"),
        format!("{visibility} use crate::owner::child::authority::*;"),
    ]
}

fn private_wrappers(visibility: &str) -> Vec<String> {
    vec![
        format!("{IMPORT}\n{visibility} struct Wide({EXTERNAL_TYPE});"),
        format!("{IMPORT}\n{visibility} struct Wide {{ inner: {EXTERNAL_TYPE} }}"),
    ]
}

fn exposed_wrappers(visibility: &str) -> Vec<String> {
    vec![
        format!("{IMPORT}\n{visibility} struct Wide({visibility} {EXTERNAL_TYPE});"),
        format!("{IMPORT}\n{visibility} struct Wide {{ {visibility} inner: {EXTERNAL_TYPE} }}"),
        format!("{IMPORT}\n{visibility} enum Wide {{ Target({EXTERNAL_TYPE}) }}"),
    ]
}

fn assert_count(target_source: &str, expected: usize) {
    assert_catalog_count(target_source, false, false, expected);
}

pub(super) fn assert_catalog_count(
    target_source: &str,
    canonical: bool,
    bridged: bool,
    expected: usize,
) {
    let files = sources(target_source);
    let parsed = files
        .iter()
        .map(|file| (file, RustSource::parse(&file.text)))
        .collect::<Vec<_>>();
    let modules = modules::discover(&parsed, &crate_catalog());
    let (file, source) = parsed
        .iter()
        .find(|(file, _)| file.relative == CONSUMER_PATH)
        .expect("consumer source");
    let mut catalog = RetainedCatalog::default();
    catalog.insert(
        path(&["owner", "child", "authority"]),
        EXTERNAL_TYPE,
        &path(&["owner", "child"]),
        canonical,
    );
    if bridged {
        catalog.alias_module(
            &path(&["owner", "child", BRIDGE_PATH_COMPONENT]),
            &path(&["owner", "child", "authority"]),
            &path(&["owner", "child"]),
        );
    }
    let facts = paths::prepare(CONSUMER_PATH, &source.tokens, &modules);
    let resolution =
        paths::resolve_prepared(CONSUMER_PATH, &source.tokens, &catalog, &modules, &facts);
    assert!(!resolution.uncertain, "fixture resolution must be certain");
    let retained = types::type_names_with(&source.tokens, &resolution);
    let mut violations = Vec::new();
    process_signal_retained_visibility::inspect_with_modules(
        file,
        source,
        &retained,
        &modules,
        &mut violations,
    );
    let actual = violations
        .iter()
        .filter(|violation| {
            violation
                .to_string()
                .contains(contract::AUTHORITY_TYPE_MESSAGE)
        })
        .count();
    assert_eq!(
        actual, expected,
        "canonical={canonical}, bridged={bridged}, source={target_source}\n{violations:#?}"
    );
}

pub(super) fn sources(target_source: &str) -> Vec<SourceFile> {
    [
        (ROOT_PATH, "mod owner;"),
        (OWNER_PATH, "mod child;"),
        (CHILD_PATH, "mod authority; mod nested;"),
        (AUTHORITY_PATH, "struct Authority;"),
        (NESTED_PATH, "mod deeper;"),
        (CONSUMER_PATH, target_source),
    ]
    .into_iter()
    .map(|(relative, text)| SourceFile {
        absolute: PathBuf::from(relative),
        relative: relative.to_owned(),
        text: text.to_owned(),
    })
    .collect()
}

pub(super) fn path(parts: &[&str]) -> Vec<String> {
    std::iter::once(CRATE_ID.to_owned())
        .chain(parts.iter().map(|part| (*part).to_owned()))
        .collect()
}

pub(super) fn crate_catalog() -> CrateCatalog {
    CrateCatalog {
        roots: BTreeSet::from([CrateRoot {
            id: CRATE_ID.to_owned(),
            source: ROOT_PATH.to_owned(),
            product: true,
        }]),
        unresolved: BTreeSet::new(),
    }
}
