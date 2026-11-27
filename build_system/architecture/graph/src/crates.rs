#[path = "crates/bindings.rs"]
mod bindings;
#[path = "crates/labels.rs"]
mod labels;
#[path = "crates/product.rs"]
mod product;
#[path = "crates/repositories.rs"]
mod repositories;
#[path = "crates/schema.rs"]
mod schema;
#[path = "crates/syntax.rs"]
mod syntax;
#[cfg(test)]
#[path = "../unit_tests/crate_catalog_tests.rs"]
mod tests;

use labels::Resolution;
use std::collections::BTreeSet;
use std::path::Path;
pub use syntax::{RuleOrigins, direct_call_receiver};
use yolo_architecture_foundation::SourceFile;
use yolo_architecture_policy_contracts::build_constants as vocabulary;
use yolo_architecture_syntax::cargo_starlark_tokens::{Token, tokenize};

const EMPTY_DEPTH: usize = 0;
const ID_SEPARATOR: &str = ":";
const KEYWORD_SPLAT_WIDTH: usize = 2;
const NEXT_TOKEN_OFFSET: usize = 1;
const RUST_SOURCE_SUFFIX: &str = ".rs";
const SINGLE_ATTRIBUTE: usize = 1;
const SRCS_ATTRIBUTE: &str = "srcs";

#[derive(Clone, Eq, Ord, PartialEq, PartialOrd)]
pub struct CrateRoot {
    pub id: String,
    pub source: String,
    pub product: bool,
}

#[derive(Default)]
pub struct Catalog {
    pub roots: BTreeSet<CrateRoot>,
    pub unresolved: BTreeSet<String>,
}

pub fn discover(files: &[SourceFile]) -> Catalog {
    let main = repositories::discover(files);
    let mut catalog = Catalog::default();
    for file in files
        .iter()
        .filter(|file| labels::build_definition(&file.relative))
    {
        inspect_file(file, &main, &mut catalog);
    }
    catalog
}

fn inspect_file(file: &SourceFile, main: &repositories::Evidence, catalog: &mut Catalog) {
    let package = Path::new(&file.relative)
        .parent()
        .and_then(Path::to_str)
        .unwrap_or_default();
    let tokens = tokenize(&file.text);
    let bindings = bindings::Bindings::parse(&tokens, &file.text);
    let origins = syntax::RuleOrigins::parse(&tokens);
    if origins.uncertain() {
        catalog.unresolved.insert(file.relative.clone());
    }
    for (open, close) in syntax::calls(&tokens) {
        let name = tokens[open - NEXT_TOKEN_OFFSET]
            .identifier()
            .unwrap_or_default();
        let rule = origins.canonical(name);
        if schema::template(file, &tokens, open, close, rule) {
            continue;
        }
        let roots = bindings::attributes(&tokens, open, close, vocabulary::CRATE_ROOT_ATTRIBUTE);
        let resolution = match roots.as_slice() {
            [root] => bindings
                .string(&tokens[root.clone()])
                .map_or(Resolution::Uncertain, |label| {
                    labels::resolve_with(package, &label, main)
                }),
            [] if labels::default_root(rule).is_some() => {
                match inferred(&tokens, open, close, package, rule, main, &bindings) {
                    Some(resolution) => resolution,
                    None if product_rule(rule) && has_keyword_splat(&tokens, open, close) => {
                        Resolution::Uncertain
                    }
                    None => continue,
                }
            }
            [] => continue,
            _ => Resolution::Uncertain,
        };
        register(
            file, &tokens, open, close, rule, resolution, &bindings, catalog,
        );
    }
}

fn inferred(
    tokens: &[Token],
    open: usize,
    close: usize,
    package: &str,
    rule: &str,
    main: &repositories::Evidence,
    bindings: &bindings::Bindings,
) -> Option<Resolution> {
    let srcs = bindings::attributes(tokens, open, close, SRCS_ATTRIBUTE);
    if srcs.is_empty() {
        return None;
    }
    let [range] = srcs.as_slice() else {
        return Some(Resolution::Uncertain);
    };
    let Some(srcs) = bindings.strings(&tokens[range.clone()]) else {
        return Some(Resolution::Uncertain);
    };
    let name = attribute_value(tokens, open, close, vocabulary::NAME_ATTRIBUTE, bindings);
    let crate_name = attribute_value(
        tokens,
        open,
        close,
        vocabulary::CRATE_NAME_ATTRIBUTE,
        bindings,
    );
    let default = labels::default_root(rule)?;
    Some(labels::infer(
        package,
        &srcs,
        default,
        name.as_deref(),
        crate_name.as_deref(),
        main,
    ))
}

#[allow(clippy::too_many_arguments)]
fn register(
    file: &SourceFile,
    tokens: &[Token],
    open: usize,
    close: usize,
    rule: &str,
    resolution: Resolution,
    bindings: &bindings::Bindings,
    catalog: &mut Catalog,
) {
    let source = match resolution {
        Resolution::Resolved(source) if source.ends_with(RUST_SOURCE_SUFFIX) => source,
        Resolution::Resolved(_) => {
            catalog.unresolved.insert(file.relative.clone());
            return;
        }
        Resolution::External => return,
        Resolution::Uncertain => {
            catalog.unresolved.insert(file.relative.clone());
            return;
        }
    };
    let target = attribute_value(tokens, open, close, vocabulary::NAME_ATTRIBUTE, bindings)
        .unwrap_or_else(|| rule.to_owned());
    let id = [
        file.relative.as_str(),
        &tokens[open].line.to_string(),
        &target,
    ]
    .join(ID_SEPARATOR);
    catalog.roots.insert(CrateRoot {
        id,
        source,
        product: product::rule_target(&tokens, open, close, rule),
    });
}

fn product_rule(rule: &str) -> bool {
    rule != vocabulary::RUST_TEST_RULE && labels::default_root(rule).is_some()
}

fn has_keyword_splat(tokens: &[Token], open: usize, close: usize) -> bool {
    tokens[open + NEXT_TOKEN_OFFSET..close]
        .windows(KEYWORD_SPLAT_WIDTH)
        .any(|pair| pair.iter().all(|token| token.symbol('*')))
}

fn attribute_value(
    tokens: &[Token],
    open: usize,
    close: usize,
    expected: &str,
    bindings: &bindings::Bindings,
) -> Option<String> {
    let ranges = bindings::attributes(tokens, open, close, expected);
    (ranges.len() == SINGLE_ATTRIBUTE)
        .then(|| bindings.string(&tokens[ranges[EMPTY_DEPTH].clone()]))
        .flatten()
}
