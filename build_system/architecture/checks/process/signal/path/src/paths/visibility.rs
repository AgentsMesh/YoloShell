use crate::visibility_fields::item_start;
use yolo_architecture_build_graph::path_syntax::{
    PARENT_COMPONENT, ROOT_COMPONENT, SELF_COMPONENT,
};
use yolo_architecture_syntax::rust_source::identifier;
use yolo_architecture_syntax::targeted_tokens::{Token, closing};

const GROUP_CLOSE: &str = ")";
const GROUP_OPEN: &str = "(";
const IN_KEYWORD: &str = "in";
const NEXT_TOKEN_OFFSET: usize = 1;
const PUBLIC_KEYWORD: &str = "pub";

pub fn scope(tokens: &[Token], index: usize, owner: &[String]) -> Vec<String> {
    let Some(public) = (item_start(tokens, index)..index)
        .find(|candidate| tokens[*candidate].identifier(PUBLIC_KEYWORD))
    else {
        return owner.to_vec();
    };
    let Some(open) = tokens
        .get(public + NEXT_TOKEN_OFFSET)
        .filter(|token| token.symbol(GROUP_OPEN))
        .map(|_| public + NEXT_TOKEN_OFFSET)
    else {
        return crate_scope(owner);
    };
    let Some(close) = closing(tokens, open, GROUP_OPEN, GROUP_CLOSE) else {
        return owner.to_vec();
    };
    let names = tokens[open + NEXT_TOKEN_OFFSET..close]
        .iter()
        .filter_map(identifier)
        .collect::<Vec<_>>();
    restricted(owner, &names)
}

fn restricted(owner: &[String], names: &[&str]) -> Vec<String> {
    let path = names.strip_prefix(&[IN_KEYWORD]).unwrap_or(names);
    match path.first().copied() {
        Some(SELF_COMPONENT) | None => owner.to_vec(),
        Some(ROOT_COMPONENT) => crate_scope(owner)
            .into_iter()
            .chain(
                path[NEXT_TOKEN_OFFSET..]
                    .iter()
                    .map(|part| (*part).to_owned()),
            )
            .collect(),
        Some(PARENT_COMPONENT) => {
            let parents = path
                .iter()
                .take_while(|part| **part == PARENT_COMPONENT)
                .count();
            let keep = owner.len().saturating_sub(parents).max(NEXT_TOKEN_OFFSET);
            owner[..keep].to_vec()
        }
        Some(_) => owner.to_vec(),
    }
}

fn crate_scope(owner: &[String]) -> Vec<String> {
    owner.first().into_iter().cloned().collect()
}
