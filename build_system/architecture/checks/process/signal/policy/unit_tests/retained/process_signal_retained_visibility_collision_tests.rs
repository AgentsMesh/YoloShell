use super::process_signal_retained_visibility_boundary_tests as fixture;
use yolo_architecture_policy_contracts::process_signal as contract;
use yolo_architecture_build_graph::modules;
use yolo_architecture_process_signal_path_analysis::paths::{self, Catalog, Resolution};
use yolo_architecture_syntax::rust_source::RustSource;

const ALIAS: &str = "Shared";
const AUTHORITY_PARTS: &[&str] = &["owner", "child", "authority"];
const CLEAN: usize = 0;
const EQUAL_VISIBILITY: &str = "pub(in crate::owner::child)";
const NARROW_VISIBILITY: &str = "pub(super)";
const REJECTED: usize = 1;
const VISIBLE_PARTS: &[&str] = &["owner", "child"];
const WIDE_VISIBILITY: &str = "pub(in crate::owner)";

#[test]
fn function_local_exact_alias_cannot_poison_an_imported_aggregate() {
    for (visibility, expected) in [(WIDE_VISIBILITY, REJECTED), (EQUAL_VISIBILITY, CLEAN)] {
        let source = format!(
            "fn poison() {{ type {ALIAS} = {}; }}\n\
             use crate::owner::child::authority::{} as {ALIAS};\n\
             {visibility} enum Exposed {{ Target({ALIAS}) }}",
            contract::RETAINED_EXACT_PROCESS,
            fixture::EXTERNAL_TYPE,
        );
        fixture::assert_catalog_count(&source, false, false, expected);
    }
}

#[test]
fn local_aggregate_record_wins_over_an_unrelated_exact_alias_name() {
    let source = format!(
        "fn poison() {{ type {ALIAS} = {}; }}\n\
         use crate::owner::child::authority::{};\n\
         {EQUAL_VISIBILITY} struct {ALIAS} {{ inner: {} }}\n\
         {WIDE_VISIBILITY} enum Exposed {{ Target({ALIAS}) }}",
        contract::RETAINED_EXACT_PROCESS,
        fixture::EXTERNAL_TYPE,
        fixture::EXTERNAL_TYPE,
    );
    fixture::assert_catalog_count(&source, false, false, REJECTED);
}

#[test]
fn field_audience_is_intersected_with_its_container_audience() {
    for (container, field, expected) in [
        (WIDE_VISIBILITY, WIDE_VISIBILITY, REJECTED),
        (WIDE_VISIBILITY, EQUAL_VISIBILITY, CLEAN),
        (WIDE_VISIBILITY, NARROW_VISIBILITY, CLEAN),
        (EQUAL_VISIBILITY, WIDE_VISIBILITY, CLEAN),
        (NARROW_VISIBILITY, WIDE_VISIBILITY, CLEAN),
    ] {
        let source = format!(
            "use crate::owner::child::authority::{};\n\
             {container} struct Holder {{ {field} target: {} }}",
            fixture::EXTERNAL_TYPE,
            fixture::EXTERNAL_TYPE,
        );
        fixture::assert_catalog_count(&source, false, false, expected);
    }
}

#[test]
fn disjoint_alias_scopes_do_not_create_file_level_ambiguity() {
    let source = format!(
        "mod first {{ use crate::owner::child::authority::{} as {ALIAS}; }}\n\
         mod second {{ use crate::owner::child::authority::{} as {ALIAS}; }}",
        contract::RETAINED_EXACT_PROCESS,
        fixture::EXTERNAL_TYPE,
    );
    let resolution = mixed_resolution(&source);
    assert!(!resolution.uncertain);
    assert!(resolution.canonical_names.contains(ALIAS));
    assert!(resolution.aggregate_names.contains_key(ALIAS));
}

#[test]
fn clean_import_binding_wins_over_a_disjoint_retained_alias_name() {
    let source = format!(
        "mod poison {{ type {ALIAS} = {}; }}\n\
         mod clean_source {{ pub struct Target; }}\n\
         mod consumer {{\n\
             use super::clean_source::Target as {ALIAS};\n\
             {WIDE_VISIBILITY} fn expose(_: {ALIAS}) {{}}\n\
         }}",
        contract::RETAINED_EXACT_PROCESS,
    );
    fixture::assert_catalog_count(&source, false, false, CLEAN);
}

#[test]
fn unresolved_same_name_reference_still_fails_closed() {
    let source = format!(
        "mod poison {{ type {ALIAS} = {}; }}\n\
         {WIDE_VISIBILITY} fn expose(_: {ALIAS}) {{}}",
        contract::RETAINED_EXACT_PROCESS,
    );
    fixture::assert_catalog_count(&source, false, false, REJECTED);
}

#[test]
fn generic_shadow_wins_over_a_disjoint_retained_alias_name() {
    let source = format!(
        "mod poison {{ type {ALIAS} = {}; }}\n\
         {WIDE_VISIBILITY} fn expose<{ALIAS}>(_: {ALIAS}) {{}}",
        contract::RETAINED_EXACT_PROCESS,
    );
    fixture::assert_catalog_count(&source, false, false, CLEAN);
}

#[test]
fn generic_shadow_cannot_hide_an_explicit_self_type_path() {
    let source = format!(
        "type {ALIAS} = {};\n\
         {WIDE_VISIBILITY} fn expose<{ALIAS}>(_: self::{ALIAS}) {{}}",
        contract::RETAINED_EXACT_PROCESS,
    );
    fixture::assert_catalog_count(&source, false, false, REJECTED);
}

#[test]
fn declaration_evidence_is_scoped_when_names_collide() {
    let clean = format!(
        "mod poison {{ type {ALIAS} = {}; }}\n\
         {WIDE_VISIBILITY} struct {ALIAS};",
        contract::RETAINED_EXACT_PROCESS,
    );
    fixture::assert_catalog_count(&clean, false, false, CLEAN);
    let retained = format!(
        "mod poison {{ type {ALIAS} = {}; }}\n\
         use crate::owner::child::authority::{};\n\
         {WIDE_VISIBILITY} struct {ALIAS} {{ {WIDE_VISIBILITY} inner: {} }}",
        contract::RETAINED_EXACT_PROCESS,
        fixture::EXTERNAL_TYPE,
        fixture::EXTERNAL_TYPE,
    );
    fixture::assert_catalog_count(&retained, false, false, REJECTED);
}

#[test]
fn same_scope_mixed_alias_evidence_remains_indeterminate() {
    let source = format!(
        "use crate::owner::child::authority::{} as {ALIAS};\n\
         use crate::owner::child::authority::{} as {ALIAS};\n\
         fn expose(_: {ALIAS}) {{}}",
        contract::RETAINED_EXACT_PROCESS,
        fixture::EXTERNAL_TYPE,
    );
    let resolution = mixed_resolution(&source);
    assert!(
        resolution.uncertain,
        "canonical={:?}, aggregate={:?}, names={:?}",
        resolution.canonical_indices, resolution.aggregate_indices, resolution.names,
    );
}

fn mixed_resolution(source_text: &str) -> Resolution {
    let files = fixture::sources(source_text);
    let parsed = files
        .iter()
        .map(|file| (file, RustSource::parse(&file.text)))
        .collect::<Vec<_>>();
    let modules = modules::discover(&parsed, &fixture::crate_catalog());
    let (_, source) = parsed
        .iter()
        .find(|(file, _)| file.relative == fixture::CONSUMER_PATH)
        .expect("consumer source");
    let mut catalog = Catalog::default();
    catalog.insert(
        fixture::path(AUTHORITY_PARTS),
        fixture::EXTERNAL_TYPE,
        &fixture::path(VISIBLE_PARTS),
        false,
    );
    catalog.insert(
        fixture::path(AUTHORITY_PARTS),
        contract::RETAINED_EXACT_PROCESS,
        &fixture::path(VISIBLE_PARTS),
        true,
    );
    let facts = paths::prepare(fixture::CONSUMER_PATH, &source.tokens, &modules);
    paths::resolve_prepared(
        fixture::CONSUMER_PATH,
        &source.tokens,
        &catalog,
        &modules,
        &facts,
    )
}
