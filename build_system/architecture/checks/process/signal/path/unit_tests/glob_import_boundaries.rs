use super::*;
use yolo_architecture_build_graph::path_syntax::ROOT_COMPONENT;
use yolo_architecture_syntax::targeted_tokens::tokenize;

const ROOT: &str = "root";
const SOURCE: &str = "source";
const NESTED: &str = "nested";
const RETAINED: &str = "RetainedExactProcess";
const NESTED_RETAINED: &str = "RetainedChild";
const VISIBILITY: usize = 0;
const IMPORT_INDEX: usize = 2;

#[test]
fn module_globs_publish_nested_modules_and_types_idempotently() {
    for module_item in [false, true] {
        let tokens = tokenize("pub use crate::source::*;");
        let scope = Scope::at(vec![ROOT.into()], &[], IMPORT_INDEX);
        let mut catalog = Catalog::default();
        catalog.insert(
            vec![ROOT.into(), SOURCE.into()],
            RETAINED,
            &scope.module,
            true,
        );
        catalog.insert(
            vec![ROOT.into(), SOURCE.into(), NESTED.into()],
            NESTED_RETAINED,
            &scope.module,
            true,
        );
        let mut aliases = Aliases::default();
        let mut result = Resolution::default();
        for expected in [true, false] {
            assert_eq!(
                import(
                    &tokens,
                    IMPORT_INDEX,
                    &[ROOT_COMPONENT.into(), SOURCE.into()],
                    std::slice::from_ref(&scope),
                    &catalog,
                    &ModuleIndex::default(),
                    &mut aliases,
                    &mut result,
                    module_item,
                    VISIBILITY
                ),
                expected
            );
        }
        assert!(result.canonical_names.contains(RETAINED));
        assert_eq!(result.type_bindings.is_empty(), !module_item);
        assert_eq!(result.module_bindings.is_empty(), !module_item);
        assert_eq!(
            aliases.candidates(&[NESTED.into()], std::slice::from_ref(&scope)),
            [vec![ROOT.to_owned(), SOURCE.to_owned(), NESTED.to_owned()]]
        );
        assert!(!result.uncertain);
    }
}

#[test]
fn inaccessible_and_missing_glob_modules_do_not_create_retained_bindings() {
    let tokens = tokenize("use crate::source::*;");
    let scope = Scope::at(vec![ROOT.into()], &[], IMPORT_INDEX);
    let mut catalog = Catalog::default();
    catalog.insert(
        vec![ROOT.into(), SOURCE.into()],
        RETAINED,
        &[ROOT.into(), SOURCE.into()],
        true,
    );
    for raw in [
        vec![ROOT_COMPONENT.into(), SOURCE.into()],
        vec![ROOT_COMPONENT.into(), NESTED.into()],
    ] {
        let mut result = Resolution::default();
        assert!(!import(
            &tokens,
            IMPORT_INDEX,
            &raw,
            std::slice::from_ref(&scope),
            &catalog,
            &ModuleIndex::default(),
            &mut Aliases::default(),
            &mut result,
            true,
            VISIBILITY
        ));
        assert!(result.names.is_empty());
        assert!(result.module_bindings.is_empty());
        assert!(!result.uncertain);
    }
}

#[test]
fn expanding_module_alias_preserves_uncertainty_without_authorizing_a_glob() {
    let tokens = tokenize("use crate::source::*;");
    let scope = Scope::at(vec![ROOT.into()], &[], IMPORT_INDEX);
    let mut catalog = Catalog::default();
    catalog.alias_module(
        &[ROOT.into(), SOURCE.into()],
        &[ROOT.into(), SOURCE.into(), NESTED.into()],
        &scope.module,
    );
    let mut result = Resolution::default();
    assert!(!import(
        &tokens,
        IMPORT_INDEX,
        &[ROOT_COMPONENT.into(), SOURCE.into()],
        std::slice::from_ref(&scope),
        &catalog,
        &ModuleIndex::default(),
        &mut Aliases::default(),
        &mut result,
        true,
        VISIBILITY
    ));
    assert!(result.uncertain);
    assert!(result.names.is_empty());
}
