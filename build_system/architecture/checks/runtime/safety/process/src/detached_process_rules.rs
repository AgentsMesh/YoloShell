use crate::constants::{
    DETACHED_STDERR_DEFINITION, DETACHED_STDERR_DEFINITION_COUNT, DETACHED_STDERR_INHERIT_PARTS,
    DETACHED_STDERR_OWNER, DETACHED_STDERR_OWNER_COUNT, DETACHED_STDERR_VARIANT_PARTS,
};
use yolo_architecture_foundation::SourceFile;
use yolo_architecture_foundation::model::Violation;
use yolo_architecture_foundation::text::joined;
use yolo_architecture_syntax::targeted_tokens::{Token, tokenize};

const POLICY_MESSAGE: &str = "inherited detached stderr is reserved for the session worker launch";
const PATH_SEPARATOR: &str = "::";
const QUALIFIED_VARIANT_TOKEN_COUNT: usize = 3;

pub fn inspect(file: &SourceFile, violations: &mut Vec<Violation>) {
    let variant = joined(DETACHED_STDERR_VARIANT_PARTS);
    let type_name = DETACHED_STDERR_INHERIT_PARTS[0];
    let tokens = tokenize(&file.text);
    let contains_type = tokens.iter().any(|token| token.identifier(type_name));
    let occurrences = tokens
        .iter()
        .filter(|token| token.identifier(&variant))
        .collect::<Vec<_>>();
    if file.relative == DETACHED_STDERR_OWNER {
        if occurrences.len() != DETACHED_STDERR_OWNER_COUNT
            || qualified_variant_count(&tokens, type_name, &variant) != DETACHED_STDERR_OWNER_COUNT
        {
            violations.push(Violation::file(&file.relative, POLICY_MESSAGE));
        }
        return;
    }
    if file.relative == DETACHED_STDERR_DEFINITION {
        if !contains_type || occurrences.len() != DETACHED_STDERR_DEFINITION_COUNT {
            violations.push(Violation::file(&file.relative, POLICY_MESSAGE));
        }
        return;
    }
    if !contains_type {
        return;
    }
    for token in occurrences {
        violations.push(Violation::line(&file.relative, token.line, POLICY_MESSAGE));
    }
}

fn qualified_variant_count(tokens: &[Token], type_name: &str, variant: &str) -> usize {
    tokens
        .windows(QUALIFIED_VARIANT_TOKEN_COUNT)
        .filter(|candidate| {
            candidate[0].identifier(type_name)
                && candidate[1].symbol(PATH_SEPARATOR)
                && candidate[2].identifier(variant)
        })
        .count()
}
