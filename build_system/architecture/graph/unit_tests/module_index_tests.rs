use super::*;
use crate::crates::{Catalog, CrateRoot};

const ROOT: &str = "core/src/lib.rs";
const CHILD: &str = "core/src/child.rs";
const MISSING: &str = "core/src/missing.rs";

fn graph(root: &str, child: &str) -> ModuleIndex {
    let files = [
        SourceFile {
            absolute: ROOT.into(),
            relative: ROOT.into(),
            text: root.into(),
        },
        SourceFile {
            absolute: CHILD.into(),
            relative: CHILD.into(),
            text: child.into(),
        },
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

#[test]
fn module_paths_and_product_and_uncertainty_state_have_independent_ownership() {
    let mut index = ModuleIndex::default();
    let root = vec!["root".to_owned()];
    let nested = vec!["root".to_owned(), "nested".to_owned()];
    assert!(index.paths_at(MISSING, 0).is_empty());
    assert!(index.blocks_at(MISSING, 0).is_empty());
    assert!(!index.module_item(MISSING, 0));
    assert!(!index.is_product(ROOT));
    assert!(index.insert(ROOT, nested.clone()));
    assert!(!index.insert(ROOT, nested.clone()));
    assert!(index.contains(&root));
    assert!(index.contains(&nested));
    assert!(index.insert(ROOT, root.clone()));
    assert_eq!(index.path_count(), 2);
    index.mark_product(ROOT);
    index.mark_uncertain(CHILD);
    index.mark_uncertain(CHILD);
    assert!(index.is_product(ROOT));
    assert_eq!(
        index.uncertain_files().collect::<Vec<_>>(),
        [&CHILD.to_owned()]
    );
    index.set_declared_crate_roots(true);
    assert!(index.uses_declared_crate_roots());
    index.set_declared_crate_roots(false);
    assert!(!index.uses_declared_crate_roots());
}

#[test]
fn inline_modules_preserve_module_ancestry_without_granting_function_scope_authority() {
    let text = "mod outer { mod inner { struct Item; fn operation() { probe(); } } }";
    let source = RustSource::parse(text);
    let probe = source
        .tokens
        .iter()
        .position(|token| token.identifier("probe"))
        .unwrap();
    let declaration = source
        .tokens
        .iter()
        .position(|token| token.identifier("Item"))
        .unwrap();
    let index = graph(text, "");
    assert_eq!(
        index.paths_at(ROOT, probe),
        [vec![
            ROOT.to_owned(),
            "outer".to_owned(),
            "inner".to_owned()
        ]]
    );
    assert!(!index.module_item(ROOT, probe));
    assert!(index.module_item(ROOT, declaration));
    assert!(!index.blocks_at(ROOT, probe).is_empty());
}

#[test]
fn include_edges_keep_test_and_disabled_code_out_of_product_authority() {
    for (source, product, uncertain) in [
        ("include!(\"child.rs\");", true, false),
        ("#[cfg(test)] include!(\"child.rs\");", false, false),
        ("#[cfg(any())] include!(\"missing.rs\");", false, false),
        ("#[cfg(test)] include!(dynamic_path);", false, false),
        ("include!(dynamic_path);", false, true),
        ("fn operation() { include!(\"child.rs\"); }", false, true),
        (
            "#[cfg(any())] mod disabled { include!(\"missing.rs\"); }",
            false,
            false,
        ),
    ] {
        let index = graph(source, "struct Child;");
        assert_eq!(index.is_product(CHILD), product, "{source}");
        assert_eq!(
            index.uncertain_files().any(|file| file == ROOT),
            uncertain,
            "{source}"
        );
    }
}

#[test]
fn missing_modules_and_include_cycles_cannot_claim_a_complete_product_graph() {
    for source in [
        "mod missing;",
        "include!(\"missing.rs\");",
        "#[path = \"missing.rs\"] mod child;",
    ] {
        assert!(graph(source, "").uncertain_files().any(|file| file == ROOT));
    }
    let cyclic = graph("include!(\"child.rs\");", "include!(\"lib.rs\");");
    assert!(cyclic.uncertain_files().any(|file| file == ROOT));
    assert!(cyclic.uncertain_files().any(|file| file == CHILD));
    let test_only = graph("#[cfg(test)] mod missing;", "");
    assert!(!test_only.uncertain_files().any(|file| file == ROOT));
}
