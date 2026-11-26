use super::process_signal_retained_visibility_boundary_tests as fixture;
use std::path::PathBuf;
use yolo_architecture_foundation::SourceFile;
use yolo_architecture_build_graph::modules;
use yolo_architecture_process_signal_path_analysis::paths::{self, Catalog};
use yolo_architecture_process_signal_path_analysis::types;
use yolo_architecture_syntax::rust_source::{RustSource, identifier};

const AUTHORITY_PATH: &str = "core/src/owner/child/nested/deeper/authority.rs";
const AUTHORITY_PARTS: &[&str] = &["owner", "child", "nested", "deeper", "authority"];
const VISIBLE_PARTS: &[&str] = &["owner", "child"];

#[test]
fn generic_shadow_cannot_hide_a_cross_file_self_path() {
    let source_text = format!(
        "mod authority; pub(in crate::owner) fn expose<authority>(_: self::authority::{}) {{}}",
        fixture::EXTERNAL_TYPE,
    );
    let mut files = fixture::sources(&source_text);
    files.push(SourceFile {
        absolute: PathBuf::from(AUTHORITY_PATH),
        relative: AUTHORITY_PATH.to_owned(),
        text: format!("pub struct {};", fixture::EXTERNAL_TYPE),
    });
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
    let facts = paths::prepare(fixture::CONSUMER_PATH, &source.tokens, &modules);
    let resolution = paths::resolve_prepared(
        fixture::CONSUMER_PATH,
        &source.tokens,
        &catalog,
        &modules,
        &facts,
    );
    let index = source
        .tokens
        .iter()
        .position(|token| identifier(token) == Some(fixture::EXTERNAL_TYPE))
        .expect("aggregate type token");
    let retained = types::type_names_with(&source.tokens, &resolution);
    assert!(!retained.contains_name(fixture::EXTERNAL_TYPE));
    assert!(resolution.aggregate_indices.contains_key(&index));
    assert!(retained.contains(&source.tokens, index));
}
