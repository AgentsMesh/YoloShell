use yolo_architecture_build_graph::path_syntax::{
    PARENT_COMPONENT, ROOT_COMPONENT, SELF_COMPONENT,
};
use yolo_architecture_syntax::rust_source::identifier;
use yolo_architecture_syntax::targeted_tokens::{Token, closing};

const GROUP_CLOSE: &str = ")";
const GROUP_OPEN: &str = "(";
const IN_KEYWORD: &str = "in";
const PATH_SEPARATOR: &str = "::";
const NEXT_TOKEN_OFFSET: usize = 1;
const PATH_TOKEN_INTERVAL: usize = 2;
const PUBLIC_KEYWORD: &str = "pub";
const ROOT_COMPONENTS: usize = 1;

pub(super) fn restricted(tokens: &[Token], public: usize, owner: &[String]) -> Option<Vec<String>> {
    let visible_from = audience(tokens, public, owner)?;
    (visible_from.len() > ROOT_COMPONENTS && visible_from.len() < owner.len())
        .then_some(visible_from)
}

pub(super) fn audience(tokens: &[Token], public: usize, owner: &[String]) -> Option<Vec<String>> {
    if !tokens.get(public)?.identifier(PUBLIC_KEYWORD) {
        return None;
    }
    if !tokens
        .get(public + NEXT_TOKEN_OFFSET)
        .is_some_and(|token| token.symbol(GROUP_OPEN))
    {
        return Some(owner.first().into_iter().cloned().collect());
    }
    let open = public + NEXT_TOKEN_OFFSET;
    let close = closing(tokens, open, GROUP_OPEN, GROUP_CLOSE)?;
    let body = &tokens[open + NEXT_TOKEN_OFFSET..close];
    let path = if body.first()?.identifier(IN_KEYWORD) {
        &body[NEXT_TOKEN_OFFSET..]
    } else {
        body
    };
    let visible_from = if exact_name(path, SELF_COMPONENT) {
        owner.to_vec()
    } else {
        restricted_path(path, owner)?
    };
    owner.starts_with(&visible_from).then_some(visible_from)
}

fn restricted_path(tokens: &[Token], owner: &[String]) -> Option<Vec<String>> {
    if tokens.first()?.identifier(PARENT_COMPONENT) {
        let names = separated_names(tokens, PARENT_COMPONENT)?;
        return parent_scope(owner, names.len());
    }
    if !tokens.first()?.identifier(ROOT_COMPONENT) {
        return None;
    }
    let names = separated_identifiers(tokens)?;
    let root = owner.first()?;
    (names.first()? == ROOT_COMPONENT).then(|| {
        std::iter::once(root.clone())
            .chain(names.into_iter().skip(NEXT_TOKEN_OFFSET))
            .collect()
    })
}

fn separated_names(tokens: &[Token], expected: &str) -> Option<Vec<String>> {
    let names = separated_identifiers(tokens)?;
    names.iter().all(|name| name == expected).then_some(names)
}

fn separated_identifiers(tokens: &[Token]) -> Option<Vec<String>> {
    let mut names = Vec::new();
    for (index, token) in tokens.iter().enumerate() {
        if index % PATH_TOKEN_INTERVAL == 0 {
            let name = identifier(token)?;
            names.push(name.to_owned());
        } else if !token.symbol(PATH_SEPARATOR) {
            return None;
        }
    }
    (!names.is_empty() && tokens.len() % PATH_TOKEN_INTERVAL == NEXT_TOKEN_OFFSET).then_some(names)
}

fn exact_name(tokens: &[Token], expected: &str) -> bool {
    tokens.len() == NEXT_TOKEN_OFFSET && tokens[0].identifier(expected)
}

fn parent_scope(owner: &[String], parents: usize) -> Option<Vec<String>> {
    let keep = owner.len().checked_sub(parents)?;
    (keep >= ROOT_COMPONENTS).then(|| owner[..keep].to_vec())
}

#[cfg(test)]
#[path = "../../../unit_tests/visibility_scope_tests.rs"]
mod tests;
