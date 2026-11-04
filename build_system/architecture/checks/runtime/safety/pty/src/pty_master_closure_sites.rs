use crate::pty_master_closure_contract as contract;
use crate::pty_master_closure_syntax as syntax;
use yolo_architecture_foundation::SourceFile;
use yolo_architecture_syntax::rust_source::{RustFunction, RustSource};
use yolo_architecture_syntax::targeted_tokens::{Kind, Token};

pub fn matches(
    site: &contract::Site,
    file: &SourceFile,
    function: Option<&RustFunction>,
    symbol: &str,
    occurrence: contract::Occurrence,
) -> bool {
    site.symbol == symbol
        && site.path == file.relative
        && site.occurrence == occurrence
        && function.is_some_and(|function| function.matches(Some(site.owner), site.function))
}

pub fn acknowledgement_relevant(
    source: &RustSource,
    index: usize,
    occurrence: Option<contract::Occurrence>,
    verifier_visible: bool,
) -> bool {
    if occurrence == Some(contract::Occurrence::Definition) {
        return source
            .function_at(index)
            .and_then(|function| function.owner.as_deref())
            == Some(contract::HOST_VERIFIER);
    }
    verifier_visible || syntax::qualified_verifier_acknowledgement(&source.tokens, index)
}

pub fn protected_constructor_owner(owner: &str) -> bool {
    [
        contract::LOCAL_PROOF,
        contract::HOST_VERIFIER,
        contract::HOST_ACK,
        contract::SETTLEMENT,
        contract::FINAL_PROOF,
    ]
    .contains(&owner)
}

pub fn method_symbol(token: &Token) -> bool {
    identifier(token).is_some_and(|name| {
        [
            contract::TAKE_LOCAL,
            contract::ACKNOWLEDGE,
            contract::ACKNOWLEDGE_HOLDER_GONE,
            contract::FINALIZE_HOST,
            contract::FINALIZE_LOCAL,
            contract::MINT_FINAL,
        ]
        .contains(&name)
    })
}

pub fn constructor_symbol(token: &Token) -> bool {
    identifier(token).is_some_and(protected_constructor_owner)
}

pub fn identifier(token: &Token) -> Option<&str> {
    match &token.kind {
        Kind::Identifier(value) => Some(value),
        _ => None,
    }
}
