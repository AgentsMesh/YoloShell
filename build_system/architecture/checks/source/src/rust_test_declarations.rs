use yolo_architecture_syntax::rust_source::RustSource;
use yolo_architecture_syntax::targeted_tokens::{Token, closing};

const ATTRIBUTE_MARKER: &str = "#";
const ATTRIBUTE_OPEN: &str = "[";
const ATTRIBUTE_CLOSE: &str = "]";
const ATTRIBUTE_ARGUMENT_OPEN: &str = "(";
const PATH_SEPARATOR: &str = "::";
const TEST_ATTRIBUTE: &str = "test";
const NEXT_TOKEN_OFFSET: usize = 1;

pub fn contains(text: &str) -> bool {
    let source = RustSource::parse(text);
    source
        .tokens
        .iter()
        .enumerate()
        .any(|(index, token)| attribute(&source.tokens, index, token).is_some_and(test_attribute))
}

fn attribute<'a>(tokens: &'a [Token], index: usize, token: &Token) -> Option<&'a [Token]> {
    if !token.symbol(ATTRIBUTE_MARKER)
        || !tokens
            .get(index + NEXT_TOKEN_OFFSET)
            .is_some_and(|next| next.symbol(ATTRIBUTE_OPEN))
    {
        return None;
    }
    let open = index + NEXT_TOKEN_OFFSET;
    let close = closing(tokens, open, ATTRIBUTE_OPEN, ATTRIBUTE_CLOSE)?;
    Some(&tokens[open + NEXT_TOKEN_OFFSET..close])
}

fn test_attribute(attribute: &[Token]) -> bool {
    let path = attribute
        .iter()
        .take_while(|token| !token.symbol(ATTRIBUTE_ARGUMENT_OPEN))
        .collect::<Vec<_>>();
    path.last()
        .is_some_and(|last| last.identifier(TEST_ATTRIBUTE))
        && (path.len() == NEXT_TOKEN_OFFSET
            || path.iter().any(|token| token.symbol(PATH_SEPARATOR)))
}
