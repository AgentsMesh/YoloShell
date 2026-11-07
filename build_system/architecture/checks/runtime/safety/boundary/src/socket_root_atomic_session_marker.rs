use super::{
    ALIAS_KEYWORD, CALL_OPEN, FUNCTION_KEYWORD, USE_KEYWORD, enclosing_function, is_rust,
    require_count, statement,
};
use yolo_architecture_foundation::SourceFile;
use yolo_architecture_foundation::model::Violation;
use yolo_architecture_policy_contracts::constants::{
    SESSION_MARKER_CAS_OWNER_FUNCTION, SESSION_MARKER_CAS_WRITER,
};
use yolo_architecture_syntax::targeted_tokens::{Token, tokenize};

const DEFINITION_PATH: &str = "core/crates/persistence/paths/src/durable/durable_json_guarded.rs";
const OWNER_PATH: &str = "core/crates/persistence/paths/src/resume.rs";
const PUB_KEYWORD: &str = "pub";
const SUPER_KEYWORD: &str = "super";
const SCOPE_OPEN: &str = "(";
const SCOPE_CLOSE: &str = ")";
const POLICY_MESSAGE: &str =
    "session marker CAS publication is restricted to the fixed resume binding marker path";
const DEFINITION_MESSAGE: &str =
    "session marker CAS writer must have exactly one paths-private definition";
const OWNER_MESSAGE: &str =
    "replace_resume_binding_marker must contain exactly one session marker CAS publication";

pub(super) fn inspect(files: &[SourceFile], violations: &mut Vec<Violation>) {
    for file in files.iter().filter(|file| is_rust(&file.relative)) {
        inspect_file(file, violations);
    }
}

fn inspect_file(file: &SourceFile, violations: &mut Vec<Violation>) {
    let tokens = tokenize(&file.text);
    let mut definitions = 0;
    let mut owner_calls = 0;
    for (index, token) in tokens.iter().enumerate() {
        if !token.identifier(SESSION_MARKER_CAS_WRITER) {
            continue;
        }
        if file.relative == DEFINITION_PATH && is_scoped_definition(&tokens, index) {
            definitions += 1;
        } else if allowed_owner_call(file, &tokens, index) {
            owner_calls += 1;
        } else {
            reject(file, token, violations);
        }
    }
    require_count(
        file,
        DEFINITION_PATH,
        definitions,
        DEFINITION_MESSAGE,
        violations,
    );
    require_count(file, OWNER_PATH, owner_calls, OWNER_MESSAGE, violations);
}

fn is_scoped_definition(tokens: &[Token], index: usize) -> bool {
    index >= 5
        && tokens[index - 5].identifier(PUB_KEYWORD)
        && tokens[index - 4].symbol(SCOPE_OPEN)
        && tokens[index - 3].identifier(SUPER_KEYWORD)
        && tokens[index - 2].symbol(SCOPE_CLOSE)
        && tokens[index - 1].identifier(FUNCTION_KEYWORD)
}

fn allowed_owner_call(file: &SourceFile, tokens: &[Token], index: usize) -> bool {
    file.relative == OWNER_PATH
        && tokens
            .get(index + 1)
            .is_some_and(|token| token.symbol(CALL_OPEN))
        && enclosing_function(tokens, index) == Some(SESSION_MARKER_CAS_OWNER_FUNCTION)
        && !statement(tokens, index)
            .iter()
            .any(|token| token.identifier(USE_KEYWORD) || token.identifier(ALIAS_KEYWORD))
}

fn reject(file: &SourceFile, token: &Token, violations: &mut Vec<Violation>) {
    violations.push(Violation::line(&file.relative, token.line, POLICY_MESSAGE));
}

#[cfg(all(test, feature = "yolo-runtime-boundary-tests"))]
#[path = "../unit_tests/session_marker_token_tests.rs"]
mod token_tests;
