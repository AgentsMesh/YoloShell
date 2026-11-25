use yolo_architecture_syntax::rust_source::identifier;
use yolo_architecture_syntax::targeted_tokens::Token;

const RUST_TREE_PREFIX: &str = "core/src/";
const RUST_CRATES_PREFIX: &str = "core/crates/";
const CRATE_SOURCE_SEPARATOR: &str = "/src/";
const RUST_SOURCE_SUFFIX: &str = ".rs";
pub const ROOT_COMPONENT: &str = "crate";
pub const SELF_COMPONENT: &str = "self";
pub const PARENT_COMPONENT: &str = "super";
const LIBRARY_MODULE: &str = "lib";
const BINARY_MODULE: &str = "main";
const DIRECTORY_MODULE: &str = "mod";
const PATH_SEPARATOR: &str = "::";
pub const BLOCK_OPEN: &str = "{";
pub const BLOCK_CLOSE: &str = "}";
const ITEM_END: &str = ";";
const FIRST_PATH_COMPONENT: usize = 1;

pub fn backward_path(tokens: &[Token], end: usize) -> Vec<String> {
    let mut start = end;
    while start >= 2
        && tokens[start - 1].symbol(PATH_SEPARATOR)
        && identifier(&tokens[start - 2]).is_some()
    {
        start -= 2;
    }
    tokens[start..=end]
        .iter()
        .filter_map(identifier)
        .map(str::to_owned)
        .collect()
}

pub fn path_candidates(raw: &[String], module: &[String]) -> Vec<Vec<String>> {
    if raw.first().is_some_and(|part| part == ROOT_COMPONENT) {
        return module.first().map_or_else(Vec::new, |root| {
            vec![
                std::iter::once(root.clone())
                    .chain(raw[FIRST_PATH_COMPONENT..].iter().cloned())
                    .collect(),
            ]
        });
    }
    if raw.first().is_some_and(|part| part == SELF_COMPONENT) {
        return vec![
            module
                .iter()
                .cloned()
                .chain(raw[1..].iter().cloned())
                .collect(),
        ];
    }
    if raw.first().is_some_and(|part| part == PARENT_COMPONENT) {
        let parents = raw
            .iter()
            .take_while(|part| part.as_str() == PARENT_COMPONENT)
            .count();
        let keep = module
            .len()
            .saturating_sub(parents)
            .max(FIRST_PATH_COMPONENT);
        return vec![
            module[..keep]
                .iter()
                .cloned()
                .chain(raw[parents..].iter().cloned())
                .collect(),
        ];
    }
    vec![
        module.iter().cloned().chain(raw.iter().cloned()).collect(),
        module
            .first()
            .into_iter()
            .cloned()
            .chain(raw.iter().cloned())
            .collect(),
    ]
}

pub fn module_path(relative: &str) -> Vec<String> {
    let source = module_source(relative);
    let source = source.strip_suffix(RUST_SOURCE_SUFFIX).unwrap_or(source);
    let mut parts = source.split('/').map(str::to_owned).collect::<Vec<_>>();
    if parts.last().is_some_and(|part| {
        matches!(
            part.as_str(),
            LIBRARY_MODULE | BINARY_MODULE | DIRECTORY_MODULE
        )
    }) {
        parts.pop();
    }
    std::iter::once(ROOT_COMPONENT.to_owned())
        .chain(parts)
        .collect()
}

fn module_source(relative: &str) -> &str {
    if let Some(source) = relative.strip_prefix(RUST_TREE_PREFIX) {
        return source;
    }
    relative
        .strip_prefix(RUST_CRATES_PREFIX)
        .and_then(|source| source.split_once(CRATE_SOURCE_SEPARATOR))
        .map_or(relative, |(_, source)| source)
}

pub fn statement_end(tokens: &[Token], start: usize) -> usize {
    tokens[start..]
        .iter()
        .position(|token| token.symbol(ITEM_END))
        .map_or(tokens.len(), |offset| start + offset)
}
