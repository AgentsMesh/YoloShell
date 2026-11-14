use super::{bindings, vocabulary};
use yolo_architecture_syntax::cargo_starlark_tokens::Token;

pub(super) fn rule_target(tokens: &[Token], open: usize, close: usize, rule: &str) -> bool {
    rule != vocabulary::RUST_TEST_RULE
        && !bindings::attributes(tokens, open, close, vocabulary::TESTONLY_ATTRIBUTE)
            .into_iter()
            .any(|value| {
                let [token] = &tokens[value] else {
                    return false;
                };
                token.identifier() == Some("True")
            })
}
