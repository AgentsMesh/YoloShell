use yolo_architecture_foundation::SourceFile;
use yolo_architecture_foundation::model::Violation;
use yolo_architecture_policy_contracts::constants::CODEX_AUTHORITY_CONSTRUCTOR;
use yolo_architecture_policy_contracts::constants::{
    CODEX_AUTHORITY_CONTROL_OWNER, CODEX_AUTHORITY_INSTALL_METHOD,
    CODEX_AUTHORITY_SUPERSEDE_LOCKED, CODEX_AUTHORITY_TAKEOVER_BINDING,
};
use yolo_architecture_syntax::rust_source::{RustFunction, RustSource};
use yolo_architecture_syntax::targeted_tokens::{Token, tokenize};

const CONTROL_PATH: &str = "core/crates/runtime/engine/worker/src/codex_hook/authority/authority_control.rs";
const TAKEOVER_PATH: &str = "core/crates/runtime/engine/worker/src/codex_hook/runtime/runtime_takeover.rs";
const NEW_FUNCTION: &str = CODEX_AUTHORITY_CONSTRUCTOR;
const FINISH_FUNCTION: &str = "finish";
const WATCH_NAME: &str = "watch";
const CHANNEL_NAME: &str = "channel";
const CHANNEL_POLICY: &str =
    "Codex authority watch channel must have one fail-closed control initialization";
const TAKEOVER_POLICY: &str =
    "Codex authority registry install must immediately supersede its predecessor";
const CHANNEL_INITIALIZATION: &str = "let (authority_tx, _) = \
    tokio::sync::watch::channel(CodexAuthorityView::StorageUnavailable);";
const TAKEOVER_SEQUENCE: &str = "let succession = takeover.install(&runtime.registration); \
    if let (Some((predecessor, lifecycle)), Some(proof)) = (predecessor, succession) { \
    predecessor.control().supersede_locked(lifecycle, proof); }";
const PREVIOUS: usize = 1;
const RECEIVER: usize = 2;
const NEXT: usize = 1;
const EXPECTED_ONE: usize = 1;

pub(super) fn inspect(files: &[&SourceFile], violations: &mut Vec<Violation>) {
    if !files
        .iter()
        .any(|file| file.relative == CONTROL_PATH || file.relative == TAKEOVER_PATH)
    {
        return;
    }
    inspect_required(
        files,
        CONTROL_PATH,
        channel_contract,
        CHANNEL_POLICY,
        violations,
    );
    inspect_required(
        files,
        TAKEOVER_PATH,
        takeover_contract,
        TAKEOVER_POLICY,
        violations,
    );
    for file in files {
        let source = RustSource::parse(&file.text);
        for (index, _) in source.tokens.iter().enumerate() {
            if watch_channel(&source.tokens, index) {
                inspect_site(
                    file,
                    &source,
                    index,
                    CONTROL_PATH,
                    channel_contract,
                    CHANNEL_POLICY,
                    violations,
                );
            } else if takeover_boundary(&source.tokens, index) {
                inspect_site(
                    file,
                    &source,
                    index,
                    TAKEOVER_PATH,
                    takeover_contract,
                    TAKEOVER_POLICY,
                    violations,
                );
            } else if supersede_call(&source.tokens, index) {
                inspect_site(
                    file,
                    &source,
                    index,
                    TAKEOVER_PATH,
                    takeover_contract,
                    TAKEOVER_POLICY,
                    violations,
                );
            }
        }
    }
}

fn inspect_required(
    files: &[&SourceFile],
    path: &str,
    contract: fn(&RustSource, &RustFunction) -> bool,
    policy: &str,
    violations: &mut Vec<Violation>,
) {
    let valid = files
        .iter()
        .find(|file| file.relative == path)
        .is_some_and(|file| {
            let source = RustSource::parse(&file.text);
            source
                .functions()
                .filter(|function| contract(&source, function))
                .count()
                == EXPECTED_ONE
        });
    if !valid {
        violations.push(Violation::file(path, policy));
    }
}

fn inspect_site(
    file: &SourceFile,
    source: &RustSource,
    index: usize,
    path: &str,
    contract: fn(&RustSource, &RustFunction) -> bool,
    policy: &str,
    violations: &mut Vec<Violation>,
) {
    let valid = file.relative == path
        && source
            .function_at(index)
            .is_some_and(|function| contract(source, function));
    if !valid {
        violations.push(Violation::line(
            &file.relative,
            source.tokens[index].line,
            policy,
        ));
    }
}

fn channel_contract(source: &RustSource, function: &RustFunction) -> bool {
    function.matches(Some(CODEX_AUTHORITY_CONTROL_OWNER), NEW_FUNCTION)
        && count(source, function, watch_channel) == EXPECTED_ONE
        && contains(source, function, CHANNEL_INITIALIZATION)
}

fn takeover_contract(source: &RustSource, function: &RustFunction) -> bool {
    function.matches(None, FINISH_FUNCTION)
        && count(source, function, takeover_boundary) == EXPECTED_ONE
        && count(source, function, supersede_call) == EXPECTED_ONE
        && contains(source, function, TAKEOVER_SEQUENCE)
}

fn count(
    source: &RustSource,
    function: &RustFunction,
    predicate: fn(&[Token], usize) -> bool,
) -> usize {
    source.tokens[function.body.clone()]
        .iter()
        .enumerate()
        .filter(|(offset, _)| predicate(&source.tokens, function.body.start + offset))
        .count()
}

fn contains(source: &RustSource, function: &RustFunction, pattern: &str) -> bool {
    let expected = tokenize(pattern);
    source.tokens[function.body.clone()]
        .windows(expected.len())
        .any(|window| {
            window
                .iter()
                .zip(&expected)
                .all(|(token, expected)| token.kind == expected.kind)
        })
}

fn watch_channel(tokens: &[Token], index: usize) -> bool {
    tokens[index].identifier(CHANNEL_NAME)
        && index >= RECEIVER
        && tokens[index - PREVIOUS].symbol("::")
        && tokens[index - RECEIVER].identifier(WATCH_NAME)
}

fn takeover_boundary(tokens: &[Token], index: usize) -> bool {
    tokens[index].identifier(CODEX_AUTHORITY_INSTALL_METHOD)
        && index >= RECEIVER
        && tokens[index - PREVIOUS].symbol(".")
        && tokens[index - RECEIVER].identifier(CODEX_AUTHORITY_TAKEOVER_BINDING)
        && call(tokens, index)
}

fn supersede_call(tokens: &[Token], index: usize) -> bool {
    tokens[index].identifier(CODEX_AUTHORITY_SUPERSEDE_LOCKED)
        && index >= PREVIOUS
        && tokens[index - PREVIOUS].symbol(".")
        && call(tokens, index)
}

fn call(tokens: &[Token], index: usize) -> bool {
    tokens
        .get(index + NEXT)
        .is_some_and(|token| token.symbol("("))
}
