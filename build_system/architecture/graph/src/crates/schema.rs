use super::{bindings, vocabulary};
use yolo_architecture_foundation::SourceFile;
use yolo_architecture_policy_contracts::entrypoint_macros::{CAPNP_DEFINITIONS, CAPNP_GENERATION};
use yolo_architecture_syntax::cargo_starlark_tokens::Token;

const OUTPUT_GROUP: &str = "OutputGroupInfo";
const SET_CONSTRUCTOR: &str = "depset";
const GENERATED_ROOT: &str = "root";
const FIRST_LINE: usize = 1;

/// A schema macro parameter is instantiated at its BUILD call site. The output
/// group's root is a generated Artifact, not a Rust process-authority crate.
/// Only these exact reviewed template forms are recognized; other calls in the
/// same files still go through ordinary root and authority resolution.
pub(super) fn template(
    file: &SourceFile,
    tokens: &[Token],
    open: usize,
    close: usize,
    rule: &str,
) -> bool {
    if !file
        .text
        .lines()
        .nth(tokens[open].line.saturating_sub(FIRST_LINE))
        .is_some_and(|line| line.starts_with(char::is_whitespace))
    {
        return false;
    }
    let roots = bindings::attributes(tokens, open, close, vocabulary::CRATE_ROOT_ATTRIBUTE);
    let [root] = roots.as_slice() else {
        return false;
    };
    let value = &tokens[root.clone()];
    match (file.relative.as_str(), rule, value) {
        (CAPNP_DEFINITIONS, vocabulary::RUST_LIBRARY_RULE, [parameter]) => {
            parameter.identifier() == Some(vocabulary::CRATE_ROOT_ATTRIBUTE)
        }
        (CAPNP_GENERATION, OUTPUT_GROUP, [constructor, open, list, generated, end_list, close]) => {
            constructor.identifier() == Some(SET_CONSTRUCTOR)
                && open.symbol('(')
                && list.symbol('[')
                && generated.identifier() == Some(GENERATED_ROOT)
                && end_list.symbol(']')
                && close.symbol(')')
        }
        _ => false,
    }
}
