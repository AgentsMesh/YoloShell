use std::collections::BTreeSet;
use std::path::PathBuf;
use yolo_architecture_foundation::SourceFile;
use yolo_architecture_policy_contracts::process_signal as contract;
use yolo_architecture_build_graph::crates::{Catalog as CrateCatalog, CrateRoot};
use yolo_architecture_build_graph::modules;
use yolo_architecture_process_signal_path_analysis::paths::{self, Catalog as RetainedCatalog};
use yolo_architecture_process_signal_path_analysis::types;
use yolo_architecture_process_signal_policy::process_signal_retained_visibility;
use yolo_architecture_syntax::rust_source::RustSource;

const CLEAN: usize = 0;
const REJECTED: usize = 1;
const ROOT_PATH: &str = "core/src/lib.rs";
const CRATE_ID: &str = ROOT_PATH;
const OWNER_PATH: &str = "core/src/owner.rs";
const CHILD_PATH: &str = "core/src/owner/child.rs";
const AUTHORITY_PATH: &str = "core/src/owner/authority.rs";
const ROOT_SOURCE: &str = "mod owner;";
const OWNER_SOURCE: &str = "mod child;";
const RETAINED_PREFIX: &str = "struct RetainedExactProcess;\n";
const EXTERNAL_OWNER_SOURCE: &str = "mod authority; mod child;";
const EXTERNAL_TYPE: &str = "Aggregate";

#[test]
fn restricted_aggregates_stay_inside_their_top_level_owner() {
    for declaration in [
        "pub(super) struct Aggregate { target: RetainedExactProcess }",
        "pub(in super) enum Aggregate { Target(RetainedExactProcess) }",
        "pub(in crate::owner) union Aggregate { target: std::mem::ManuallyDrop<RetainedExactProcess> }",
    ] {
        assert_type_count(CHILD_PATH, OWNER_SOURCE, declaration, CLEAN);
    }
}

#[test]
fn aggregate_visibility_cannot_cross_the_owner_boundary() {
    for declaration in [
        "pub(crate) struct Aggregate { target: RetainedExactProcess }",
        "pub(in crate) struct Aggregate { target: RetainedExactProcess }",
        "pub struct Aggregate { target: RetainedExactProcess }",
    ] {
        assert_type_count(CHILD_PATH, OWNER_SOURCE, declaration, REJECTED);
    }
    let declaration = "pub(super) struct Aggregate { target: RetainedExactProcess }";
    assert_type_count(OWNER_PATH, declaration, declaration, REJECTED);
}

#[test]
fn exact_authorities_and_aliases_still_require_approved_sites() {
    assert_source_count(
        CHILD_PATH,
        OWNER_SOURCE,
        "pub(super) struct RetainedExactProcess;",
        REJECTED,
    );
    assert_type_count(
        CHILD_PATH,
        OWNER_SOURCE,
        "pub(super) type Aggregate = RetainedExactProcess;",
        REJECTED,
    );
}

fn assert_type_count(target: &str, owner: &str, declaration: &str, expected: usize) {
    let target_source = format!("{RETAINED_PREFIX}{declaration}");
    assert_source_count(target, owner, &target_source, expected);
}

pub(super) fn assert_child_count(target_source: &str, expected: usize) {
    assert_source_count(CHILD_PATH, OWNER_SOURCE, target_source, expected);
}

pub(super) fn assert_external_count(target_source: &str, canonical: bool, expected: usize) {
    let files = vec![
        source(ROOT_PATH, ROOT_SOURCE),
        source(OWNER_PATH, EXTERNAL_OWNER_SOURCE),
        source(AUTHORITY_PATH, "struct Authority;"),
        source(CHILD_PATH, target_source),
    ];
    let parsed = files
        .iter()
        .map(|file| (file, RustSource::parse(&file.text)))
        .collect::<Vec<_>>();
    let modules = modules::discover(&parsed, &crate_catalog());
    let (file, source) = parsed
        .iter()
        .find(|(file, _)| file.relative == CHILD_PATH)
        .expect("target source");
    let mut catalog = RetainedCatalog::default();
    catalog.insert(
        vec![
            CRATE_ID.to_owned(),
            "owner".to_owned(),
            "authority".to_owned(),
        ],
        EXTERNAL_TYPE,
        &[CRATE_ID.to_owned(), "owner".to_owned()],
        canonical,
    );
    let facts = paths::prepare(CHILD_PATH, &source.tokens, &modules);
    let resolution =
        paths::resolve_prepared(CHILD_PATH, &source.tokens, &catalog, &modules, &facts);
    assert!(!resolution.uncertain, "external fixture must resolve");
    if canonical && target_source.starts_with("use ") {
        assert!(
            resolution.canonical_names.contains(EXTERNAL_TYPE),
            "canonical import was not bound: {:?}",
            resolution.canonical_names
        );
    }
    let retained = types::type_names_with(&source.tokens, &resolution);
    let mut violations = Vec::new();
    process_signal_retained_visibility::inspect_with_modules(
        file,
        source,
        &retained,
        &modules,
        &mut violations,
    );
    let actual = violation_count(&violations);
    assert_eq!(
        actual, expected,
        "canonical={canonical}, source={target_source}\n{violations:#?}"
    );
}

fn assert_source_count(target: &str, owner: &str, target_source: &str, expected: usize) {
    let files = sources(target, owner, target_source);
    let parsed = files
        .iter()
        .map(|file| (file, RustSource::parse(&file.text)))
        .collect::<Vec<_>>();
    let modules = modules::discover(&parsed, &crate_catalog());
    let (file, source) = parsed
        .iter()
        .find(|(file, _)| file.relative == target)
        .expect("target source");
    let retained = types::type_names(&source.tokens);
    let mut violations = Vec::new();
    process_signal_retained_visibility::inspect_with_modules(
        file,
        source,
        &retained,
        &modules,
        &mut violations,
    );
    let actual = violation_count(&violations);
    assert_eq!(actual, expected, "source={target_source}\n{violations:#?}");
}

fn violation_count(violations: &[yolo_architecture_foundation::model::Violation]) -> usize {
    violations
        .iter()
        .filter(|violation| {
            violation
                .to_string()
                .contains(contract::AUTHORITY_TYPE_MESSAGE)
        })
        .count()
}

fn sources(target: &str, owner: &str, target_source: &str) -> Vec<SourceFile> {
    let mut values = vec![source(ROOT_PATH, ROOT_SOURCE)];
    if target == OWNER_PATH {
        values.push(source(OWNER_PATH, target_source));
    } else {
        values.push(source(OWNER_PATH, owner));
        values.push(source(target, target_source));
    }
    values
}

fn source(relative: &str, text: &str) -> SourceFile {
    SourceFile {
        absolute: PathBuf::from(relative),
        relative: relative.to_owned(),
        text: text.to_owned(),
    }
}

fn crate_catalog() -> CrateCatalog {
    CrateCatalog {
        roots: BTreeSet::from([CrateRoot {
            id: CRATE_ID.to_owned(),
            source: ROOT_PATH.to_owned(),
            product: true,
        }]),
        unresolved: BTreeSet::new(),
    }
}
