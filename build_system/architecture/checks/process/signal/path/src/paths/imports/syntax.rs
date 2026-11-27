use yolo_architecture_build_graph::path_syntax::BLOCK_CLOSE;
use yolo_architecture_syntax::rust_source::identifier;
use yolo_architecture_syntax::targeted_tokens::Token;

const ALIAS_BINDING_OFFSET: usize = 2;
const ALIAS_KEYWORD: &str = "as";
const ITEM_END: &str = ";";
const ITEM_SEPARATOR: &str = ",";
const NEXT_TOKEN_OFFSET: usize = 1;

pub(super) fn target(tokens: &[Token], index: usize, end: usize) -> bool {
    tokens.get(index + NEXT_TOKEN_OFFSET).is_none_or(|next| {
        index + NEXT_TOKEN_OFFSET >= end
            || next.identifier(ALIAS_KEYWORD)
            || next.symbol(ITEM_SEPARATOR)
            || next.symbol(ITEM_END)
            || next.symbol(BLOCK_CLOSE)
    })
}

pub(super) fn binding_name(tokens: &[Token], index: usize, raw: &[String]) -> String {
    if tokens
        .get(index + NEXT_TOKEN_OFFSET)
        .is_some_and(|token| token.identifier(ALIAS_KEYWORD))
    {
        return tokens
            .get(index + ALIAS_BINDING_OFFSET)
            .and_then(identifier)
            .map(str::to_owned)
            .unwrap_or_default();
    }
    raw.last().cloned().unwrap_or_default()
}
