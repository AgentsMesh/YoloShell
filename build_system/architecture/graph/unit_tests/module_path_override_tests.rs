use super::*;
use crate::crates::{Catalog, CrateRoot};

const ROOT: &str = "core/src/lib.rs";
const MODULE_ENTRY: &str = "core/unit_tests/entry.rs";
const ADJACENT_CHILD: &str = "core/unit_tests/child.rs";
const STEM_CHILD: &str = "core/unit_tests/entry/child.rs";
const INLINE_CHILD: &str = "core/unit_tests/nested/child.rs";
const GRANDCHILD: &str = "core/unit_tests/child/leaf.rs";

fn graph(root: &str, entry: &str) -> ModuleIndex {
    let files = [
        source(ROOT, root),
        source(MODULE_ENTRY, entry),
        source(ADJACENT_CHILD, "mod leaf;"),
        source(STEM_CHILD, "pub struct Wrong;"),
        source(INLINE_CHILD, "pub struct Inline;"),
        source(GRANDCHILD, "pub struct Leaf;"),
    ];
    let catalog = Catalog {
        roots: BTreeSet::from([CrateRoot {
            id: ROOT.into(),
            source: ROOT.into(),
            product: true,
        }]),
        unresolved: BTreeSet::new(),
    };
    let sources = files
        .iter()
        .map(|file| (file, RustSource::parse(&file.text)))
        .collect::<Vec<_>>();
    discover(&sources, &catalog)
}

fn source(path: &str, text: &str) -> SourceFile {
    SourceFile {
        absolute: path.into(),
        relative: path.into(),
        text: text.into(),
    }
}

#[test]
fn overridden_module_searches_its_parent_directory_not_its_filename_stem() {
    let index = graph(
        "#[path = \"../unit_tests/entry.rs\"] mod cases;",
        "mod child;",
    );
    assert!(index.is_product(ADJACENT_CHILD));
    assert!(!index.is_product(STEM_CHILD));
    assert!(index.is_product(GRANDCHILD));
    assert_eq!(
        index.paths_at(ADJACENT_CHILD, 0),
        [vec![
            ROOT.to_owned(),
            "cases".to_owned(),
            "child".to_owned()
        ]]
    );
    assert!(index.uncertain_files().next().is_none());
}

#[test]
fn overridden_test_module_does_not_promote_children_into_product_authority() {
    let index = graph(
        "#[cfg(test)] #[path = \"../unit_tests/entry.rs\"] mod cases;",
        "mod child;",
    );
    for path in [MODULE_ENTRY, ADJACENT_CHILD, GRANDCHILD] {
        assert!(!index.paths_at(path, 0).is_empty());
        assert!(!index.is_product(path));
    }
    assert!(index.paths_at(STEM_CHILD, 0).is_empty());
    assert!(index.uncertain_files().next().is_none());
}

#[test]
fn overridden_parent_keeps_inline_module_nesting() {
    let index = graph(
        "#[path = \"../unit_tests/entry.rs\"] mod cases;",
        "mod nested { mod child; }",
    );
    assert!(index.is_product(INLINE_CHILD));
    assert!(!index.is_product(ADJACENT_CHILD));
    assert!(index.uncertain_files().next().is_none());
}

#[test]
fn explicit_descendant_paths_remain_relative_to_the_overridden_source() {
    let index = graph(
        "#[path = \"../unit_tests/entry.rs\"] mod cases;",
        "#[path = \"child.rs\"] mod renamed;",
    );
    assert!(index.is_product(ADJACENT_CHILD));
    assert!(!index.is_product(STEM_CHILD));
    // child.rs is itself overridden, so its mod leaf searches adjacent to child.rs.
    assert!(!index.is_product(GRANDCHILD));
    assert!(index.uncertain_files().any(|path| path == ADJACENT_CHILD));
}
