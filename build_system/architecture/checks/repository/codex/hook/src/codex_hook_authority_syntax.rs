use yolo_architecture_policy_contracts::constants::*;
use yolo_architecture_syntax::rust_source::{RustFunction, RustSource, identifier};
use yolo_architecture_syntax::targeted_tokens::{Token, tokenize};

pub(super) const CONTROL_OWNER: &str = CODEX_AUTHORITY_CONTROL_OWNER;
pub(super) const RUNTIME_OWNER: &str = "CodexHookRuntime";
const SNAPSHOT_PROOF: &str = "CodexSnapshotProof";
const SUCCESSION_PROOF: &str = "CodexSuccessionProof";
const PROOF_PARAMETER: &str = "proof";
const WATCH_SURFACE: &str = "authority_tx.send(value); authority_tx.send_modify(modify); authority_tx.send_replace(value); authority_tx.send_if_modified(modify);";
const WATCH_SUBSCRIPTION: &str = "authority_tx.subscribe();";
const INITIAL_BODY: &str = "{ self.publish_proven(proof, view) }";
const PROVEN_BODY: &str = "{ proof.linearize_publication().context(super::super::constants::CODEX_STORAGE_ERROR)?; self.authority_tx.send_replace(view); Ok(()) }";
const LOST_BODY: &str = "{ self.authority_tx.send_if_modified(|view| { if matches!(view, CodexAuthorityView::AuthorityLost | CodexAuthorityView::Superseded) { return false; } *view = CodexAuthorityView::AuthorityLost; true }); }";
const STORAGE_BODY: &str = "{ self.authority_tx.send_if_modified(|view| { if matches!(view, CodexAuthorityView::AuthorityLost | CodexAuthorityView::StorageUnavailable | CodexAuthorityView::Superseded) { return false; } *view = CodexAuthorityView::StorageUnavailable; true }); }";
const SUPERSEDED_BODY: &str = "{ self.authority_tx.send_replace(CodexAuthorityView::Superseded); }";
const SUPERSEDE_LOCKED_BODY: &str =
    "{ lifecycle.supersede_authorized(&proof); self.publish_superseded(proof); }";
const FAILED_TAKEOVER_BODY: &str = "{ if lifecycle.prove_snapshot_current().is_ok() || !lifecycle.latch_authority_lost() { return; } self.publish_terminal_authority_lost(); }";
const LIFECYCLE_BODY: &str = "{ if let Some(view) = lifecycle.authority_view() { return self.registration.control().publish_proven(proof, view); } Ok(()) }";
const ACTIVE_BODY: &str =
    "{ self.registration.control().publish_proven(proof, CodexAuthorityView::Active(state)) }";
const RUNTIME_LOST_BODY: &str =
    "{ self.registration.control().publish_terminal_authority_lost(); }";
const RUNTIME_STORAGE_BODY: &str =
    "{ self.registration.control().publish_terminal_storage_unavailable(); }";
const NEXT: usize = 1;
const SECOND: usize = 2;
const THIRD: usize = 3;
const FOURTH: usize = 4;

pub(super) const REQUIRED_CONTRACTS: &[(&str, &str)] = &[
    (CONTROL_OWNER, CODEX_AUTHORITY_PUBLISH_INITIAL),
    (CONTROL_OWNER, CODEX_AUTHORITY_PUBLISH_PROVEN),
    (CONTROL_OWNER, CODEX_AUTHORITY_PUBLISH_TERMINAL_LOST),
    (CONTROL_OWNER, CODEX_AUTHORITY_PUBLISH_TERMINAL_STORAGE),
    (CONTROL_OWNER, CODEX_AUTHORITY_PUBLISH_SUPERSEDED),
    (CONTROL_OWNER, CODEX_AUTHORITY_SUPERSEDE_LOCKED),
    (CONTROL_OWNER, CODEX_AUTHORITY_CONTAIN_FAILED_TAKEOVER),
    (RUNTIME_OWNER, CODEX_AUTHORITY_PUBLISH_LIFECYCLE),
    (RUNTIME_OWNER, CODEX_AUTHORITY_PUBLISH_ACTIVE),
    (RUNTIME_OWNER, CODEX_AUTHORITY_PUBLISH_LOST),
    (RUNTIME_OWNER, CODEX_AUTHORITY_PUBLISH_STORAGE),
];

pub(super) fn contract(source: &RustSource, function: &RustFunction) -> bool {
    let owner = function.owner.as_deref();
    let name = function.name.as_str();
    let body = match (owner, name) {
        (Some(CONTROL_OWNER), CODEX_AUTHORITY_PUBLISH_INITIAL) => INITIAL_BODY,
        (Some(CONTROL_OWNER), CODEX_AUTHORITY_PUBLISH_PROVEN) => PROVEN_BODY,
        (Some(CONTROL_OWNER), CODEX_AUTHORITY_PUBLISH_TERMINAL_LOST) => LOST_BODY,
        (Some(CONTROL_OWNER), CODEX_AUTHORITY_PUBLISH_TERMINAL_STORAGE) => STORAGE_BODY,
        (Some(CONTROL_OWNER), CODEX_AUTHORITY_PUBLISH_SUPERSEDED) => SUPERSEDED_BODY,
        (Some(CONTROL_OWNER), CODEX_AUTHORITY_SUPERSEDE_LOCKED) => SUPERSEDE_LOCKED_BODY,
        (Some(CONTROL_OWNER), CODEX_AUTHORITY_CONTAIN_FAILED_TAKEOVER) => FAILED_TAKEOVER_BODY,
        (Some(RUNTIME_OWNER), CODEX_AUTHORITY_PUBLISH_LIFECYCLE) => LIFECYCLE_BODY,
        (Some(RUNTIME_OWNER), CODEX_AUTHORITY_PUBLISH_ACTIVE) => ACTIVE_BODY,
        (Some(RUNTIME_OWNER), CODEX_AUTHORITY_PUBLISH_LOST) => RUNTIME_LOST_BODY,
        (Some(RUNTIME_OWNER), CODEX_AUTHORITY_PUBLISH_STORAGE) => RUNTIME_STORAGE_BODY,
        _ => return false,
    };
    let proof_ok = match name {
        CODEX_AUTHORITY_PUBLISH_INITIAL
        | CODEX_AUTHORITY_PUBLISH_PROVEN
        | CODEX_AUTHORITY_PUBLISH_LIFECYCLE
        | CODEX_AUTHORITY_PUBLISH_ACTIVE => signature_has_type(source, function, SNAPSHOT_PROOF),
        CODEX_AUTHORITY_PUBLISH_SUPERSEDED | CODEX_AUTHORITY_SUPERSEDE_LOCKED => {
            signature_has_type(source, function, SUCCESSION_PROOF)
        }
        _ => true,
    };
    proof_ok && function_block(source, function).is_some_and(|tokens| exact(tokens, body))
}

pub(super) fn allowed_gated_call(
    source: &RustSource,
    function: &RustFunction,
    index: usize,
    callee: &str,
) -> bool {
    let caller = function.name.as_str();
    let allowed = match callee {
        CODEX_AUTHORITY_PUBLISH_PROVEN => {
            matches!(
                caller,
                CODEX_AUTHORITY_PUBLISH_INITIAL
                    | CODEX_AUTHORITY_PUBLISH_LIFECYCLE
                    | CODEX_AUTHORITY_PUBLISH_ACTIVE
            ) && signature_has_type(source, function, SNAPSHOT_PROOF)
                && call_starts_with(source, index, PROOF_PARAMETER)
        }
        CODEX_AUTHORITY_PUBLISH_SUPERSEDED => {
            caller == CODEX_AUTHORITY_SUPERSEDE_LOCKED
                && signature_has_type(source, function, SUCCESSION_PROOF)
                && call_starts_with(source, index, PROOF_PARAMETER)
        }
        _ => false,
    };
    allowed && contract(source, function)
}

pub(super) fn mutation_method(tokens: &[Token], index: usize) -> Option<&str> {
    let surface = tokenize(WATCH_SURFACE);
    let sender = surface.first()?;
    if index < SECOND
        || !tokens[index - NEXT].symbol(".")
        || tokens[index - SECOND].kind != sender.kind
        || !tokens
            .get(index + NEXT)
            .is_some_and(|token| token.symbol("("))
    {
        return None;
    }
    let method = identifier(&tokens[index])?;
    let declared = matches!(
        method,
        CODEX_AUTHORITY_SEND_REPLACE | CODEX_AUTHORITY_SEND_IF_MODIFIED
    ) || surface.windows(FOURTH).any(|window| {
        window[0].kind == sender.kind
            && window[1].symbol(".")
            && window[2].kind == tokens[index].kind
            && window[3].symbol("(")
    });
    declared.then_some(method)
}

pub(super) fn sender_escapes(tokens: &[Token], index: usize) -> bool {
    let surface = tokenize(WATCH_SURFACE);
    let sender = &surface[usize::default()];
    if index == usize::default()
        || !tokens[index - NEXT].symbol(".")
        || tokens[index].kind != sender.kind
    {
        return false;
    }
    let method = index + SECOND;
    if !tokens
        .get(index + NEXT)
        .is_some_and(|token| token.symbol("."))
        || !tokens
            .get(index + THIRD)
            .is_some_and(|token| token.symbol("("))
    {
        return true;
    }
    if mutation_method(tokens, method).is_some() {
        return false;
    }
    let subscription = tokenize(WATCH_SUBSCRIPTION);
    !subscription
        .get(SECOND)
        .is_some_and(|expected| tokens[method].kind == expected.kind)
}

fn signature_has_type(source: &RustSource, function: &RustFunction, expected: &str) -> bool {
    let tokens = &source.tokens[function.body.clone()];
    let prefix = if expected == SNAPSHOT_PROOF { "&" } else { ":" };
    tokens.iter().enumerate().any(|(index, token)| {
        token.identifier(expected)
            && index
                .checked_sub(NEXT)
                .and_then(|previous| tokens.get(previous))
                .is_some_and(|token| token.symbol(prefix))
    })
}

fn call_starts_with(source: &RustSource, index: usize, parameter: &str) -> bool {
    let tokens = &source.tokens;
    tokens
        .get(index + NEXT)
        .is_some_and(|token| token.symbol("("))
        && tokens
            .get(index + SECOND)
            .is_some_and(|token| token.identifier(parameter))
        && tokens
            .get(index + THIRD)
            .is_some_and(|token| token.symbol(",") || token.symbol(")"))
}

fn function_block<'a>(source: &'a RustSource, function: &RustFunction) -> Option<&'a [Token]> {
    let tokens = &source.tokens[function.body.clone()];
    let open = tokens.iter().position(|token| token.symbol("{"))?;
    Some(&tokens[open..])
}

fn exact(tokens: &[Token], expected: &str) -> bool {
    let expected = tokenize(expected);
    tokens.len() == expected.len()
        && tokens
            .iter()
            .zip(&expected)
            .all(|(token, expected)| token.kind == expected.kind)
}

#[cfg(test)]
#[path = "../unit_tests/codex_hook_syntax_boundaries.rs"]
mod tests;
