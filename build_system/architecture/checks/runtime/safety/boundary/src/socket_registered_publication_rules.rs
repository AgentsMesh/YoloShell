use super::{enclosing_function, is_rust, is_socket_namespace_source, statement};
use yolo_architecture_foundation::{SourceFile, model::Violation};
use yolo_architecture_syntax::targeted_tokens::{Token, tokenize};

#[path = "socket_registered_publication_contracts.rs"]
mod contracts;
use contracts::*;

const COMMA: &str = ",";
const FUNCTION: &str = "fn";
const QUALIFICATION: &str = "::";
const NATIVE_ATTRIBUTE: &str = stringify!(native_xattr);
const PRIVATE_DEFINITION: &str = "pub(super) fn";
const USE: &str = "use";
const POLICY_MESSAGE: &str = "registered socket namespace publication must use its exact admission owner and fenced create-only authority";
const CONTRACT_MESSAGE: &str =
    "registered socket namespace publication contract is missing or duplicated";

pub(super) fn inspect(files: &[SourceFile], violations: &mut Vec<Violation>) {
    let mut counts = vec![0_usize; CONTRACTS.len()];
    let present = files.iter().any(|file| {
        CONTRACTS
            .iter()
            .any(|contract| contract.path == file.relative)
    });
    for file in files
        .iter()
        .filter(|file| is_rust(&file.relative) && is_socket_namespace_source(&file.relative))
    {
        let tokens = normalized(&file.text);
        for (index, token) in tokens.iter().enumerate() {
            if BROAD_WRITERS.iter().any(|writer| token.identifier(writer)) {
                reject(file, token, violations);
            }
            for (contract_index, contract) in CONTRACTS.iter().enumerate() {
                if !token.identifier(contract.callee) || exempt_definition(file, &tokens, index) {
                    continue;
                }
                let exact = file.relative == contract.path
                    && enclosing_function(&tokens, index) == Some(contract.function)
                    && contract
                        .calls
                        .iter()
                        .any(|call| matches_call(&tokens, index, contract.callee, call));
                if exact {
                    counts[contract_index] += 1;
                } else if relevant_reference(file, &tokens, index, contract) {
                    reject(file, token, violations);
                }
            }
        }
    }
    if present {
        for (contract, count) in CONTRACTS.iter().zip(counts) {
            if count != contract.count {
                violations.push(Violation::file(contract.path, CONTRACT_MESSAGE));
            }
        }
    }
}

fn normalized(text: &str) -> Vec<Token> {
    tokenize(text)
        .into_iter()
        .filter(|token| !token.symbol(COMMA))
        .collect()
}

fn matches_call(tokens: &[Token], index: usize, callee: &str, text: &str) -> bool {
    let expected = normalized(text);
    let Some(offset) = expected.iter().position(|token| token.identifier(callee)) else {
        return false;
    };
    let Some(start) = index.checked_sub(offset) else {
        return false;
    };
    tokens
        .get(start..start + expected.len())
        .is_some_and(|candidate| {
            candidate
                .iter()
                .zip(expected)
                .all(|(actual, expected)| actual.kind == expected.kind)
        })
}

fn exempt_definition(file: &SourceFile, tokens: &[Token], index: usize) -> bool {
    let prefix = normalized(PRIVATE_DEFINITION);
    index >= prefix.len()
        && tokens[index - 1].identifier(FUNCTION)
        && tokens[index - prefix.len()..index]
            .iter()
            .zip(prefix)
            .all(|(actual, expected)| actual.kind == expected.kind)
        && ((file.relative == CERTIFICATE_PATH && tokens[index].identifier(INSTALL))
            || (file.relative == ATTRIBUTE_PATH && tokens[index].identifier(CREATE)))
}

fn relevant_reference(
    file: &SourceFile,
    tokens: &[Token],
    index: usize,
    contract: &Contract,
) -> bool {
    if contract.callee == CREATE {
        let expression = statement(tokens, index);
        return [CERTIFICATE_PATH, ATTRIBUTE_PATH].contains(&file.relative.as_str())
            || (expression.iter().any(|token| token.identifier(USE))
                && expression
                    .iter()
                    .any(|token| token.identifier(NATIVE_ATTRIBUTE)))
            || (index >= 2
                && tokens[index - 1].symbol(QUALIFICATION)
                && tokens[index - 2].identifier(NATIVE_ATTRIBUTE));
    }
    if ["acquire", "sync"].contains(&contract.callee) {
        return file.relative == contract.path;
    }
    true
}

fn reject(file: &SourceFile, token: &Token, violations: &mut Vec<Violation>) {
    violations.push(Violation::line(&file.relative, token.line, POLICY_MESSAGE));
}

#[cfg(all(test, feature = "yolo-runtime-boundary-tests"))]
#[path = "../unit_tests/socket_registered_publication_tests.rs"]
mod tests;
