use yolo_architecture_foundation::SourceFile;
use yolo_architecture_foundation::model::Violation;
use yolo_architecture_policy_contracts::constants::*;
use yolo_architecture_syntax::rust_source::{RustFunction, RustSource};

#[path = "codex_hook_authority_syntax.rs"]
mod syntax;
use syntax::*;
#[path = "codex_hook_authority_boundaries.rs"]
mod boundaries;

const SCOPE_PREFIX: &str = "core/crates/runtime/engine/worker/src/codex_hook/";
const OWNER_PATH: &str = "core/crates/runtime/engine/worker/src/codex_hook/runtime/runtime_authority.rs";
const RUST_SUFFIX: &str = ".rs";
const FUNCTION_KEYWORD: &str = "fn";
const EXPECTED_OCCURRENCES: usize = 1;
const PREVIOUS_TOKEN: usize = 1;
const DIRECT_POLICY: &str =
    "Codex authority watch mutation is restricted to control publication owners";
const OWNER_POLICY: &str =
    "Codex authority watch mutation is restricted to its exact control publication owners";
const SENDER_POLICY: &str =
    "Codex authority watch sender may only be observed or used by control publication owners";
const PROOF_POLICY: &str = "Codex authority Active and Exhausted publication must pass \
    CodexSnapshotProof through the sealed control publisher";
const SUCCESSION_POLICY: &str =
    "Codex authority Superseded publication requires registry-issued CodexSuccessionProof";
const CONTRACT_POLICY: &str = "Codex authority control publication contract changed unexpectedly";

pub(super) fn inspect(files: &[SourceFile], violations: &mut Vec<Violation>) {
    let scoped = files
        .iter()
        .filter(|file| {
            file.relative.starts_with(SCOPE_PREFIX) && file.relative.ends_with(RUST_SUFFIX)
        })
        .collect::<Vec<_>>();
    if scoped.is_empty() {
        return;
    }
    match scoped.iter().find(|file| file.relative == OWNER_PATH) {
        Some(owner) => inspect_contract(owner, violations),
        None => violations.push(Violation::file(OWNER_PATH, CONTRACT_POLICY)),
    }
    boundaries::inspect(&scoped, violations);
    for file in scoped {
        inspect_file(file, violations);
    }
}

fn inspect_file(file: &SourceFile, violations: &mut Vec<Violation>) {
    let source = RustSource::parse(&file.text);
    for (index, token) in source.tokens.iter().enumerate() {
        if sender_escapes(&source.tokens, index) {
            violations.push(Violation::line(&file.relative, token.line, SENDER_POLICY));
        }
        if let Some(method) = mutation_method(&source.tokens, index) {
            inspect_mutation(file, &source, index, method, violations);
        }
        if definition(&source.tokens, index) {
            continue;
        }
        if token.identifier(CODEX_AUTHORITY_PUBLISH_PROVEN) {
            inspect_gated_call(file, &source, index, PROOF_POLICY, violations);
        } else if token.identifier(CODEX_AUTHORITY_PUBLISH_SUPERSEDED) {
            inspect_gated_call(file, &source, index, SUCCESSION_POLICY, violations);
        }
    }
}

fn definition(tokens: &[yolo_architecture_syntax::targeted_tokens::Token], index: usize) -> bool {
    index
        .checked_sub(PREVIOUS_TOKEN)
        .and_then(|previous| tokens.get(previous))
        .is_some_and(|token| token.identifier(FUNCTION_KEYWORD))
}

fn inspect_mutation(
    file: &SourceFile,
    source: &RustSource,
    index: usize,
    method: &str,
    violations: &mut Vec<Violation>,
) {
    let token = &source.tokens[index];
    if file.relative != OWNER_PATH {
        violations.push(Violation::line(&file.relative, token.line, DIRECT_POLICY));
        return;
    }
    let allowed = source
        .function_at(index)
        .is_some_and(|function| allowed_mutation(source, function, method));
    if !allowed {
        violations.push(Violation::line(&file.relative, token.line, OWNER_POLICY));
    }
}

fn allowed_mutation(source: &RustSource, function: &RustFunction, method: &str) -> bool {
    let valid_method = match function.name.as_str() {
        CODEX_AUTHORITY_PUBLISH_PROVEN | CODEX_AUTHORITY_PUBLISH_SUPERSEDED => {
            method == CODEX_AUTHORITY_SEND_REPLACE
        }
        CODEX_AUTHORITY_PUBLISH_TERMINAL_LOST | CODEX_AUTHORITY_PUBLISH_TERMINAL_STORAGE => {
            method == CODEX_AUTHORITY_SEND_IF_MODIFIED
        }
        _ => false,
    };
    function.owner.as_deref() == Some(CONTROL_OWNER) && valid_method && contract(source, function)
}

fn inspect_gated_call(
    file: &SourceFile,
    source: &RustSource,
    index: usize,
    policy: &str,
    violations: &mut Vec<Violation>,
) {
    let callee = match policy {
        PROOF_POLICY => CODEX_AUTHORITY_PUBLISH_PROVEN,
        SUCCESSION_POLICY => CODEX_AUTHORITY_PUBLISH_SUPERSEDED,
        _ => return,
    };
    let valid = file.relative == OWNER_PATH
        && source
            .function_at(index)
            .is_some_and(|function| allowed_gated_call(source, function, index, callee));
    if !valid {
        violations.push(Violation::line(
            &file.relative,
            source.tokens[index].line,
            policy,
        ));
    }
}

fn inspect_contract(file: &SourceFile, violations: &mut Vec<Violation>) {
    let source = RustSource::parse(&file.text);
    let valid = REQUIRED_CONTRACTS
        .iter()
        .all(|(owner, name)| exact_owner(&source, owner, name));
    if !valid {
        violations.push(Violation::file(&file.relative, CONTRACT_POLICY));
    }
}

fn exact_owner(source: &RustSource, owner: &str, name: &str) -> bool {
    let owners = source
        .functions()
        .filter(|function| function.matches(Some(owner), name))
        .collect::<Vec<&RustFunction>>();
    owners.len() == EXPECTED_OCCURRENCES
        && owners
            .first()
            .is_some_and(|function| contract(source, function))
}
