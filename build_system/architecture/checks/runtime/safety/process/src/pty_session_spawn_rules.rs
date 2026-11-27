use crate::rust_process_spawn_contract::PTY_SESSION_SOURCE_PREFIXES;
use yolo_architecture_foundation::SourceFile;
use yolo_architecture_foundation::model::Violation;
use yolo_architecture_syntax::rust_source::RustSource;

const RUST_SOURCE_SUFFIX: &str = ".rs";
const SPAWN_METHOD: &str = "spawn";
const POLICY_MESSAGE: &str =
    "PTY-session child creation must use the centralized waitable spawn boundary";

pub fn inspect(file: &SourceFile, violations: &mut Vec<Violation>) {
    if !pty_session_source(&file.relative) {
        return;
    }
    let source = RustSource::parse(&file.text);
    for (index, token) in source.tokens.iter().enumerate() {
        if token.identifier(SPAWN_METHOD)
            && index > usize::default()
            && source.tokens[index - 1].symbol(".")
        {
            violations.push(Violation::line(&file.relative, token.line, POLICY_MESSAGE));
        }
    }
}

fn pty_session_source(relative: &str) -> bool {
    PTY_SESSION_SOURCE_PREFIXES
        .iter()
        .any(|prefix| relative.starts_with(prefix))
        && relative.ends_with(RUST_SOURCE_SUFFIX)
}
