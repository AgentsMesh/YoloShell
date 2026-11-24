use super::*;
use std::collections::BTreeSet;
use yolo_architecture_foundation::SourceFile;
use yolo_architecture_build_graph::{
    crates::{Catalog, CrateRoot},
    modules,
};
use yolo_architecture_syntax::rust_source::RustSource;

const ROOT: &str = "core/src/lib.rs";

#[test]
fn extern_self_alias_requires_every_keyword_and_a_named_alias() {
    for (text, expected) in [
        ("extern crate self as Alias;", 1),
        ("external crate self as Alias;", 0),
        ("extern other self as Alias;", 0),
        ("extern crate other as Alias;", 0),
        ("extern crate self alias Alias;", 0),
        ("extern crate self as 5;", 0),
        ("extern crate self as", 0),
        ("fn local() { extern crate self as Alias; }", 0),
    ] {
        let file = SourceFile {
            absolute: ROOT.into(),
            relative: ROOT.into(),
            text: text.into(),
        };
        let source = RustSource::parse(text);
        let catalog = Catalog {
            roots: BTreeSet::from([CrateRoot {
                id: ROOT.into(),
                source: ROOT.into(),
                product: true,
            }]),
            unresolved: BTreeSet::new(),
        };
        let modules = modules::discover(&[(&file, RustSource::parse(text))], &catalog);
        let mut aliases = Aliases::default();
        let mut result = Resolution::default();
        scan(ROOT, &source.tokens, &modules, &mut aliases, &mut result);
        assert_eq!(result.module_bindings.len(), expected, "{text}");
    }
}
