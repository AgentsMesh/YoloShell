use super::process_signal_retained_visibility_boundary_tests as fixture;
use std::collections::BTreeSet;
use yolo_architecture_build_graph::modules;
use yolo_architecture_build_graph::path_syntax::backward_path;
use yolo_architecture_process_signal_path_analysis::paths::{self, Catalog, Resolution};
use yolo_architecture_process_signal_path_analysis::types;
use yolo_architecture_process_signal_policy::process_signal_retained_visibility;
use yolo_architecture_syntax::rust_source::{RustSource, identifier};
use yolo_architecture_syntax::targeted_tokens::Token;

const BLOCK_OPEN: &str = "{";
const FOR_KEYWORD: &str = "for";
const IMPLEMENTATIONS_PER_OWNER: usize = 2;
const NEXT_TOKEN_OFFSET: usize = 1;
const STRUCT_KEYWORD: &str = "struct";
const WRAPPERS: &[&str] = &["RootWrapper", "ParentWrapper", "SelfWrapper"];

#[test]
fn rooted_deref_owners_bind_their_declaration_indices() {
    let source_text = r#"
struct RetainedExactProcess;
struct RootWrapper;
mod parent {
    struct ParentWrapper;
    mod child {
        struct SelfWrapper;
        impl Deref for self::SelfWrapper {
            type Target = crate::owner::child::nested::deeper::RetainedExactProcess;
        }
        impl DerefMut for self::SelfWrapper {}
        impl Deref for super::ParentWrapper {
            type Target = crate::owner::child::nested::deeper::RetainedExactProcess;
        }
        impl DerefMut for super::ParentWrapper {}
        impl Deref for crate::owner::child::nested::deeper::RootWrapper {
            type Target = crate::owner::child::nested::deeper::RetainedExactProcess;
        }
        impl DerefMut for crate::owner::child::nested::deeper::RootWrapper {}
    }
}
"#;
    let (source, resolution) = resolve(source_text);
    let retained = types::type_names_with(&source.tokens, &resolution);
    let exported = process_signal_retained_visibility::declared_types(&source, &retained)
        .into_iter()
        .map(|(index, _, _)| index)
        .collect::<BTreeSet<_>>();
    for name in WRAPPERS {
        let declaration = struct_index(&source.tokens, name);
        let owners = owner_indices(&source.tokens, name);
        assert_eq!(owners.len(), IMPLEMENTATIONS_PER_OWNER, "owner={name}");
        for owner in owners {
            assert_eq!(
                resolution.declaration_indices.get(&owner),
                Some(&BTreeSet::from([declaration])),
                "owner={name}, token={owner}",
            );
        }
        assert_eq!(retained.declaration_canonical(declaration), Some(false));
        assert!(exported.contains(&declaration), "owner={name}");
    }
}

#[test]
fn rooted_clean_references_ignore_disjoint_exact_aliases() {
    let source_text = r#"
struct RetainedExactProcess;
mod poison {
    type RootShared = RetainedExactProcess;
    type ParentShared = RetainedExactProcess;
    type SelfShared = RetainedExactProcess;
}
struct RootShared;
fn root(_: self::RootShared) {}
mod parent {
    struct ParentShared;
    mod child {
        struct SelfShared;
        fn expose(
            _: self::SelfShared,
            _: super::ParentShared,
            _: crate::owner::child::nested::deeper::RootShared,
        ) {}
    }
}
"#;
    let (source, resolution) = resolve(source_text);
    let retained = types::type_names_with(&source.tokens, &resolution);
    for name in ["RootShared", "ParentShared", "SelfShared"] {
        let declaration = struct_index(&source.tokens, name);
        let reference = rooted_reference(&source.tokens, name);
        assert_eq!(
            resolution.declaration_indices.get(&reference),
            Some(&BTreeSet::from([declaration])),
            "reference={name}",
        );
        assert_eq!(retained.declaration_canonical(declaration), None);
        assert!(
            !retained.contains(&source.tokens, reference),
            "reference={name}"
        );
    }
}

fn resolve(source_text: &str) -> (RustSource, Resolution) {
    let files = fixture::sources(source_text);
    let parsed = files
        .iter()
        .map(|file| (file, RustSource::parse(&file.text)))
        .collect::<Vec<_>>();
    let modules = modules::discover(&parsed, &fixture::crate_catalog());
    let source = RustSource::parse(source_text);
    let facts = paths::prepare(fixture::CONSUMER_PATH, &source.tokens, &modules);
    let resolution = paths::resolve_prepared(
        fixture::CONSUMER_PATH,
        &source.tokens,
        &Catalog::default(),
        &modules,
        &facts,
    );
    (source, resolution)
}

fn struct_index(tokens: &[Token], name: &str) -> usize {
    tokens
        .windows(NEXT_TOKEN_OFFSET + NEXT_TOKEN_OFFSET)
        .position(|pair| {
            pair[0].identifier(STRUCT_KEYWORD) && identifier(&pair[NEXT_TOKEN_OFFSET]) == Some(name)
        })
        .expect("struct declaration")
}

fn owner_indices(tokens: &[Token], name: &str) -> Vec<usize> {
    tokens
        .iter()
        .enumerate()
        .filter(|(_, token)| token.identifier(FOR_KEYWORD))
        .filter_map(|(index, _)| {
            let end = tokens[index..]
                .iter()
                .position(|token| token.symbol(BLOCK_OPEN))?
                + index;
            (index + NEXT_TOKEN_OFFSET..end)
                .rev()
                .find(|candidate| identifier(&tokens[*candidate]) == Some(name))
        })
        .collect()
}

fn rooted_reference(tokens: &[Token], name: &str) -> usize {
    tokens
        .iter()
        .enumerate()
        .find(|(index, token)| {
            identifier(token) == Some(name)
                && backward_path(tokens, *index).len() > NEXT_TOKEN_OFFSET
        })
        .map(|(index, _)| index)
        .expect("rooted reference")
}
