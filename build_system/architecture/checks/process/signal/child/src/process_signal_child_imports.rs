use yolo_architecture_policy_contracts::process_signal as contract;
use yolo_architecture_syntax::targeted_tokens::Token;

const ITEM_END: &str = ";";
const USE_KEYWORD: &str = "use";
const ALIAS_KEYWORD: &str = "as";
const PATH_SEPARATOR: &str = "::";

pub(super) fn use_statement(tokens: &[Token], index: usize) -> Option<&[Token]> {
    let start = tokens[..index]
        .iter()
        .rposition(|token| token.symbol(ITEM_END))
        .map_or(0, |position| position + 1);
    if !tokens[start..=index]
        .iter()
        .any(|token| token.identifier(USE_KEYWORD))
    {
        return None;
    }
    let end = tokens[index..]
        .iter()
        .position(|token| token.symbol(ITEM_END))
        .map_or(tokens.len(), |offset| index + offset);
    Some(&tokens[start..end])
}

pub(super) fn unsafe_import(tokens: &[Token], index: usize, statement: &[Token]) -> bool {
    !tokens
        .get(index + 1)
        .is_some_and(|token| token.symbol(PATH_SEPARATOR))
        && !statement
            .iter()
            .any(|token| token.identifier(contract::LIBC_NAMESPACE))
        && (tokens
            .get(index + 1)
            .is_some_and(|token| token.identifier(ALIAS_KEYWORD))
            || !namespace_only_import(tokens, index))
}

fn namespace_only_import(tokens: &[Token], import: usize) -> bool {
    let mut usages = tokens.iter().enumerate().filter_map(|(index, token)| {
        (index != import
            && token.identifier(contract::KILL_METHOD)
            && use_statement(tokens, index).is_none())
        .then_some(index)
    });
    let Some(first) = usages.next() else {
        return false;
    };
    std::iter::once(first).chain(usages).all(|index| {
        tokens
            .get(index + 1)
            .is_some_and(|token| token.symbol(PATH_SEPARATOR))
    })
}
