use crate::targeted_tokens::Token;

pub(crate) const ASSIGNMENT: &str = "=";
pub(crate) const LIST_CLOSE: &str = "]";
pub(crate) const LIST_OPEN: &str = "[";
pub(crate) const SEPARATOR: &str = ",";
const COMMENT: &str = "#";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Malformed {
    pub line: usize,
}

pub(crate) fn list_start(tokens: &[Token], assignment: &str) -> Result<usize, Malformed> {
    let start = tokens
        .iter()
        .position(|token| token.identifier(assignment))
        .ok_or(Malformed { line: 1 })?;
    if !tokens
        .get(start + 1)
        .is_some_and(|token| token.symbol(ASSIGNMENT))
        || !tokens
            .get(start + 2)
            .is_some_and(|token| token.symbol(LIST_OPEN))
    {
        return Err(Malformed {
            line: tokens[start].line,
        });
    }
    Ok(start + 3)
}

pub(crate) fn without_comments(tokens: Vec<Token>) -> Vec<Token> {
    let mut result = Vec::with_capacity(tokens.len());
    let mut comment_line = None;
    for token in tokens {
        if let Some(line) = comment_line {
            if token.line == line {
                continue;
            }
            comment_line = None;
        }
        if token.symbol(COMMENT) {
            comment_line = Some(token.line);
            continue;
        }
        result.push(token);
    }
    result
}
