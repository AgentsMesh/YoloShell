use crate::{required_field_constants::*, required_field_syntax as syntax};
use yolo_architecture_foundation::{SourceFile, model::Violation};
use yolo_architecture_syntax::targeted_tokens::{Token, tokenize};

pub(super) fn inspect(file: &SourceFile, violations: &mut Vec<Violation>) {
    if ![OFFER_OWNER, RECORD_OWNER, PROFILE_OWNER, ENVELOPE_OWNER].contains(&file.relative.as_str())
    {
        return;
    }
    let tokens = tokenize(&file.text);
    match file.relative.as_str() {
        OFFER_OWNER => check_field(file, &tokens, OFFER_TYPE, None, PROFILE_FIELD, violations),
        RECORD_OWNER => {
            for variant in STATE_VARIANTS {
                check_field(
                    file,
                    &tokens,
                    STATE_TYPE,
                    Some(variant),
                    PROFILE_FIELD,
                    violations,
                );
            }
            check_field(file, &tokens, LEASE_TYPE, None, PROFILE_FIELD, violations);
        }
        PROFILE_OWNER => check_profile(file, &tokens, violations),
        ENVELOPE_OWNER => {
            for name in ENVELOPE_TYPES {
                check_field(file, &tokens, name, None, PAYLOAD_FIELD, violations);
                let denied = syntax::declaration(&tokens, name).is_some_and(|item| {
                    syntax::direct_attribute_contains(
                        &tokens,
                        &item.attributes,
                        SERDE,
                        DENY_UNKNOWN,
                    )
                });
                if !denied {
                    violations.push(Violation::file(&file.relative, ENVELOPE_MESSAGE));
                }
            }
        }
        _ => unreachable!(),
    }
}

fn check_field(
    file: &SourceFile,
    tokens: &[Token],
    owner: &str,
    variant: Option<&str>,
    name: &str,
    violations: &mut Vec<Violation>,
) {
    let valid = syntax::declaration(tokens, owner).is_some_and(|item| {
        if syntax::attribute_contains(tokens, &item.attributes, SERDE, INFERRED_FIELD_OPTIONS) {
            return false;
        }
        let body = match variant {
            Some(variant) => syntax::variant(tokens, item.body, variant),
            None => Some(item.body),
        };
        body.and_then(|body| syntax::field(tokens, body, name))
            .is_some_and(|field| {
                let expected = if name == PROFILE_FIELD {
                    PROFILE_TYPE
                } else {
                    PAYLOAD_TYPE
                };
                syntax::direct_type(tokens, &field, expected)
                    && !syntax::attribute_contains(
                        tokens,
                        &field.attributes,
                        SERDE,
                        INFERRED_FIELD_OPTIONS,
                    )
            })
    });
    if !valid {
        violations.push(Violation::file(&file.relative, FIELD_MESSAGE));
    }
}

fn check_profile(file: &SourceFile, tokens: &[Token], violations: &mut Vec<Violation>) {
    let derived = syntax::declaration(tokens, PROFILE_TYPE).is_none_or(|item| {
        syntax::attribute_contains(tokens, &item.attributes, DERIVE, &[DEFAULT_TRAIT])
    });
    let implementation = tokens.iter().enumerate().find_map(|(index, token)| {
        if !token.identifier("impl") {
            return None;
        }
        let head = tokens[index..].split(|token| token.symbol("{")).next()?;
        (head.iter().any(|token| token.identifier(DEFAULT_TRAIT))
            && head.iter().any(|token| token.identifier("for"))
            && head.iter().any(|token| token.identifier(PROFILE_TYPE)))
        .then_some(token.line)
    });
    if derived || implementation.is_some() {
        violations.push(Violation::line(
            &file.relative,
            implementation.unwrap_or(FIRST_LINE),
            PROFILE_MESSAGE,
        ));
    }
}

#[cfg(test)]
#[path = "../unit_tests/required_field.rs"]
mod tests;
