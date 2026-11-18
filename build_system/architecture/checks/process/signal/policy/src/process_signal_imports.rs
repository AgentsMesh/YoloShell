use yolo_architecture_foundation::SourceFile;
use yolo_architecture_foundation::model::Violation;
use yolo_architecture_policy_contracts::process_signal as contract;
use yolo_architecture_process_signal_foundation::process_exec_contract as exec_contract;
use yolo_architecture_syntax::rust_source::{RustSource, identifier};

const ITEM_END: &str = ";";
const USE_KEYWORD: &str = "use";
const SELF_KEYWORD: &str = "self";
const ALIAS_KEYWORD: &str = "as";
const GLOB_SYMBOL: &str = "*";

pub fn inspect(file: &SourceFile, source: &RustSource, violations: &mut Vec<Violation>) {
    for (index, token) in source.tokens.iter().enumerate() {
        if direct_libc_alias(&source.tokens[index..]) {
            violation(file, token, contract::RAW_SIGNAL_MESSAGE, violations);
        }
        if !token.identifier(USE_KEYWORD) {
            continue;
        }
        inspect_use(file, &source.tokens[index..], violations);
    }
}

fn inspect_use(
    file: &SourceFile,
    tail: &[yolo_architecture_syntax::targeted_tokens::Token],
    violations: &mut Vec<Violation>,
) {
    let end = tail
        .iter()
        .position(|token| token.symbol(ITEM_END))
        .unwrap_or(tail.len());
    let statement = &tail[..end];
    let Some(libc) = statement
        .iter()
        .find(|token| token.identifier(contract::LIBC_NAMESPACE))
    else {
        return;
    };
    let sensitive = statement.iter().any(|token| {
        identifier(token).is_some_and(|name| {
            contract::RAW_LIBC_SIGNAL_NAMES.contains(&name)
                || name == exec_contract::EXECVEAT_FUNCTION
                || matches!(name, contract::LIBC_SYSCALL | contract::LIBC_PRCTL)
        })
    });
    let glob = statement.iter().any(|token| token.symbol(GLOB_SYMBOL));
    let aliases_namespace = statement.windows(2).any(namespace_alias);
    if sensitive || glob || aliases_namespace {
        let message = if statement
            .iter()
            .any(|token| token.identifier(exec_contract::EXECVEAT_FUNCTION))
        {
            exec_contract::RAW_EXEC_MESSAGE
        } else {
            contract::RAW_SIGNAL_MESSAGE
        };
        violation(file, libc, message, violations);
    }
}

fn namespace_alias(tokens: &[yolo_architecture_syntax::targeted_tokens::Token]) -> bool {
    matches!(tokens, [name, alias, ..]
        if (name.identifier(contract::LIBC_NAMESPACE) || name.identifier(SELF_KEYWORD))
            && alias.identifier(ALIAS_KEYWORD))
}

fn direct_libc_alias(tokens: &[yolo_architecture_syntax::targeted_tokens::Token]) -> bool {
    matches!(tokens, [name, alias, ..]
        if name.identifier(contract::LIBC_NAMESPACE) && alias.identifier(ALIAS_KEYWORD))
}

fn violation(
    file: &SourceFile,
    token: &yolo_architecture_syntax::targeted_tokens::Token,
    message: &str,
    violations: &mut Vec<Violation>,
) {
    violations.push(Violation::line(&file.relative, token.line, message));
}
