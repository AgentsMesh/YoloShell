use std::collections::BTreeSet;
use yolo_architecture_policy_contracts::process_signal as contract;
use yolo_architecture_syntax::rust_source::identifier;
use yolo_architecture_syntax::targeted_tokens::Token;

const USE_KEYWORD: &str = "use";
const TYPE_KEYWORD: &str = "type";
const AS_KEYWORD: &str = "as";
const SELF_KEYWORD: &str = "self";
const ASSIGNMENT: &str = "=";
const ITEM_END: &str = ";";

pub fn exact_names(tokens: &[Token]) -> BTreeSet<String> {
    let mut names = BTreeSet::from([contract::RETAINED_EXACT_PROCESS.to_owned()]);
    while extend(tokens, &mut names, &BTreeSet::new()) {}
    names
}

pub fn extend(tokens: &[Token], names: &mut BTreeSet<String>, qualified: &BTreeSet<usize>) -> bool {
    let mut changed = false;
    for (index, token) in tokens.iter().enumerate() {
        if token.identifier(USE_KEYWORD) {
            let end = statement_end(tokens, index);
            for alias in import_aliases(&tokens[index..end], names) {
                changed |= names.insert(alias);
            }
        } else if token.identifier(TYPE_KEYWORD)
            && let Some(alias) = type_alias(tokens, index, names, qualified)
        {
            changed |= names.insert(alias);
        }
    }
    changed
}

fn statement_end(tokens: &[Token], start: usize) -> usize {
    tokens[start..]
        .iter()
        .position(|token| token.symbol(ITEM_END))
        .map_or(tokens.len(), |offset| start + offset)
}

fn import_aliases(tokens: &[Token], names: &BTreeSet<String>) -> Vec<String> {
    tokens
        .iter()
        .enumerate()
        .filter_map(|(index, token)| {
            if !token.identifier(AS_KEYWORD) {
                return None;
            }
            let source = tokens[..index].iter().rev().find_map(identifier)?;
            let alias = tokens.get(index + 1).and_then(identifier)?;
            let sensitive = names.contains(source)
                || source == SELF_KEYWORD
                    && tokens[..index]
                        .iter()
                        .any(|token| identifier(token).is_some_and(|name| names.contains(name)));
            sensitive.then(|| alias.to_owned())
        })
        .collect()
}

fn type_alias(
    tokens: &[Token],
    index: usize,
    names: &BTreeSet<String>,
    qualified: &BTreeSet<usize>,
) -> Option<String> {
    let alias = tokens.get(index + 1).and_then(identifier)?;
    let end = statement_end(tokens, index);
    let assignment = tokens[index..end]
        .iter()
        .position(|token| token.symbol(ASSIGNMENT))?
        + index;
    tokens[assignment + 1..end]
        .iter()
        .enumerate()
        .any(|(offset, token)| {
            qualified.contains(&(assignment + 1 + offset))
                || identifier(token).is_some_and(|name| names.contains(name))
        })
        .then(|| alias.to_owned())
}
