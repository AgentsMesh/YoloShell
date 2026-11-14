use super::reachability_cfg::{self, Truth};
use super::reachability_config::Configuration;
use yolo_architecture_syntax::rust_source::RustSource;
use yolo_architecture_syntax::targeted_tokens::{Token, closing, tokenize};

const ATTRIBUTE: &str = "#";
const INNER: &str = "!";
const OPEN: &str = "[";
const CLOSE: &str = "]";
const CFG: &str = "cfg";
const PREDICATE_OPEN: &str = "(";
const PREDICATE_CLOSE: &str = ")";
const ENABLED: &str = "#[cfg(all())]";
const DISABLED: &str = "#[cfg(any())]";
const NEXT: usize = 1;
const PREDICATE_OFFSET: usize = 3;
const BLOCK_OPEN: &str = "{";
const BLOCK_CLOSE: &str = "}";

pub(super) fn configure(tokens: &[Token], configuration: &Configuration) -> Option<RustSource> {
    let mut result = RustSource::parse("");
    result.tokens = configure_scope(tokens, configuration)?;
    Some(result)
}

fn configure_scope(tokens: &[Token], configuration: &Configuration) -> Option<Vec<Token>> {
    let mut result = Vec::new();
    let mut cursor = usize::default();
    while cursor < tokens.len() {
        if tokens[cursor].symbol(BLOCK_OPEN)
            && let Some(end) = closing(tokens, cursor, BLOCK_OPEN, BLOCK_CLOSE)
        {
            result.push(tokens[cursor].clone());
            result.extend(
                configure_scope(&tokens[cursor + NEXT..end], configuration).unwrap_or_default(),
            );
            result.push(tokens[end].clone());
            cursor = end + NEXT;
            continue;
        }
        if let Some((end, truth, inner)) = attribute(tokens, cursor, configuration) {
            if inner && truth == Truth::No {
                return None;
            }
            let replacement = match truth {
                Truth::Yes => Some(ENABLED),
                Truth::No => Some(DISABLED),
                Truth::Unknown => None,
            };
            if let Some(replacement) = replacement {
                result.extend(tokenize(replacement).into_iter().map(|mut token| {
                    token.line = tokens[cursor].line;
                    token
                }));
            } else {
                result.extend_from_slice(&tokens[cursor..end]);
            }
            cursor = end;
        } else {
            result.push(tokens[cursor].clone());
            cursor += NEXT;
        }
    }
    Some(result)
}

fn attribute(
    tokens: &[Token],
    cursor: usize,
    configuration: &Configuration,
) -> Option<(usize, Truth, bool)> {
    if !tokens[cursor].symbol(ATTRIBUTE) {
        return None;
    }
    let inner = tokens.get(cursor + NEXT)?.symbol(INNER);
    let open = cursor + NEXT + usize::from(inner);
    if !tokens.get(open)?.symbol(OPEN) || !tokens.get(open + NEXT)?.identifier(CFG) {
        return None;
    }
    let close = closing(tokens, open, OPEN, CLOSE)?;
    let predicate_open = open + NEXT + NEXT;
    let predicate_close = close.checked_sub(NEXT)?;
    if !tokens.get(predicate_open)?.symbol(PREDICATE_OPEN)
        || !tokens.get(predicate_close)?.symbol(PREDICATE_CLOSE)
    {
        return None;
    }
    let truth = reachability_cfg::evaluate(
        &tokens[open + PREDICATE_OFFSET..predicate_close],
        configuration,
    );
    Some((close + NEXT, truth, inner))
}
