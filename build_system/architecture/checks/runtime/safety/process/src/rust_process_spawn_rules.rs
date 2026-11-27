use crate::rust_process_spawn_contract::*;
use yolo_architecture_foundation::SourceFile;
use yolo_architecture_foundation::model::Violation;
use yolo_architecture_syntax::rust_source::{RustFunction, RustSource, identifier};
use yolo_architecture_syntax::targeted_tokens::Token;

pub fn inspect(file: &SourceFile, violations: &mut Vec<Violation>) {
    if !runtime_library_source(&file.relative) {
        return;
    }
    let source = RustSource::parse(&file.text);
    let factories = command_factories(&source.tokens);
    for (index, token) in source.tokens.iter().enumerate() {
        if !token.identifier(SPAWN_METHOD)
            || !source
                .tokens
                .get(index.wrapping_sub(1))
                .is_some_and(|token| token.symbol("."))
        {
            continue;
        }
        let Some(function) = source.function_at(index) else {
            continue;
        };
        let body = &source.tokens[function.body.clone()];
        let spawn = index - function.body.start;
        if process_command_spawn(body, spawn, &factories)
            && !guard_precedes(body, spawn)
            && !boundary_spawn_method(file, function)
        {
            violations.push(Violation::line(&file.relative, token.line, POLICY_MESSAGE));
        }
    }
}

fn boundary_spawn_method(file: &SourceFile, function: &RustFunction) -> bool {
    file.relative == SPAWN_BOUNDARY_SOURCE && function.owner.as_deref() == Some(SPAWN_BOUNDARY_TYPE)
}

fn runtime_library_source(relative: &str) -> bool {
    yolo_architecture_source_policy::source_policy::is_rust_source(relative)
        && !PTY_SESSION_SOURCE_PREFIXES
            .iter()
            .any(|prefix| relative.starts_with(prefix))
        && !BINARY_SOURCE_PREFIXES
            .iter()
            .any(|prefix| relative.starts_with(prefix))
        && !BINARY_SOURCE_FILES.contains(&relative)
}

fn process_command_spawn(tokens: &[Token], spawn: usize, factories: &[String]) -> bool {
    let statement = statement_start(tokens, spawn);
    command_expression(&tokens[statement..spawn], factories)
        || receiver_binding(tokens, spawn, factories)
}

fn receiver_binding(tokens: &[Token], spawn: usize, factories: &[String]) -> bool {
    let receiver = spawn
        .checked_sub(2)
        .and_then(|index| identifier(&tokens[index]));
    let Some(receiver) = receiver else {
        return false;
    };
    if parameter_is_command(tokens, receiver) {
        return true;
    }
    let mut cursor = 0;
    while cursor < spawn {
        if tokens[cursor].identifier("let") {
            let binding = cursor
                + usize::from(
                    tokens
                        .get(cursor + 1)
                        .is_some_and(|token| token.identifier("mut")),
                )
                + 1;
            if tokens.get(binding).and_then(identifier) != Some(receiver) {
                cursor += 1;
                continue;
            }
            let end = tokens[cursor..spawn]
                .iter()
                .position(|token| token.symbol(";"))
                .map_or(spawn, |offset| cursor + offset);
            if command_expression(&tokens[cursor..end], factories) {
                return true;
            }
        }
        cursor += 1;
    }
    false
}

fn command_expression(tokens: &[Token], factories: &[String]) -> bool {
    tokens.windows(3).any(|window| {
        window[0].identifier(COMMAND_TYPE) && window[1].symbol("::") && window[2].identifier("new")
    }) || tokens.iter().enumerate().any(|(index, token)| {
        identifier(token).is_some_and(|name| {
            (name.ends_with(COMMAND_FACTORY_SUFFIX) || factories.iter().any(|item| item == name))
                && tokens.get(index + 1).is_some_and(|next| next.symbol("("))
        })
    })
}

fn parameter_is_command(tokens: &[Token], receiver: &str) -> bool {
    tokens.iter().enumerate().any(|(index, token)| {
        if !token.identifier(receiver)
            || !tokens.get(index + 1).is_some_and(|token| token.symbol(":"))
        {
            return false;
        }
        let parameter_type = &tokens[index + 2..];
        parameter_type
            .first()
            .is_some_and(|token| token.identifier(COMMAND_TYPE))
            || parameter_type
                .first()
                .is_some_and(|token| token.symbol("&"))
                && (parameter_type
                    .get(1)
                    .is_some_and(|token| token.identifier(COMMAND_TYPE))
                    || parameter_type
                        .get(1)
                        .is_some_and(|token| token.identifier("mut"))
                        && parameter_type
                            .get(2)
                            .is_some_and(|token| token.identifier(COMMAND_TYPE)))
    })
}

fn command_factories(tokens: &[Token]) -> Vec<String> {
    tokens
        .windows(5)
        .filter_map(|window| {
            (window[0].identifier("fn")
                && window[2].symbol("(")
                && window.iter().any(|token| token.identifier(COMMAND_TYPE)))
            .then(|| identifier(&window[1]).map(str::to_owned))
            .flatten()
        })
        .collect()
}

fn guard_precedes(tokens: &[Token], spawn: usize) -> bool {
    tokens[..spawn].windows(2).any(|window| {
        SPAWN_GUARDS.iter().any(|guard| window[0].identifier(guard)) && window[1].symbol("(")
    })
}

fn statement_start(tokens: &[Token], end: usize) -> usize {
    (0..end)
        .rev()
        .find(|index| tokens[*index].symbol(";") || tokens[*index].symbol("{"))
        .map_or(0, |index| index + 1)
}
