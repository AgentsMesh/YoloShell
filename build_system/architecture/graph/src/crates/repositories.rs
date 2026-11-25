use super::bindings::Bindings;
use yolo_architecture_foundation::SourceFile;
use yolo_architecture_policy_contracts::build_constants as vocabulary;
use yolo_architecture_syntax::cargo_starlark_tokens::{Token, tokenize};

const ABSOLUTE_LABEL_PREFIX: &str = "//";
const APPARENT_REPOSITORY_PREFIX: &str = "@";
const CANONICAL_REPOSITORY_PREFIX: &str = "@@";
const MODULE_DEFINITION: &str = "MODULE.bazel";
const MODULE_FUNCTION: &str = "module";
const NEXT_TOKEN_OFFSET: usize = 1;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum Evidence {
    Known(String),
    Unknown,
}

pub(super) enum Repository<'a> {
    Main(&'a str),
    External,
    Uncertain,
}

pub(super) fn discover(files: &[SourceFile]) -> Evidence {
    let definitions = files
        .iter()
        .filter(|file| file.relative == MODULE_DEFINITION)
        .collect::<Vec<_>>();
    let [definition] = definitions.as_slice() else {
        return Evidence::Unknown;
    };
    let tokens = tokenize(&definition.text);
    let bindings = Bindings::parse(&tokens, &definition.text);
    let names = super::syntax::calls(&tokens)
        .into_iter()
        .filter(|(open, _)| tokens[*open - NEXT_TOKEN_OFFSET].identifier() == Some(MODULE_FUNCTION))
        .map(|(open, close)| repository_name(&tokens, open, close, &bindings))
        .collect::<Vec<_>>();
    match names.as_slice() {
        [Some(name)] if !name.is_empty() => Evidence::Known(name.clone()),
        _ => Evidence::Unknown,
    }
}

fn repository_name(
    tokens: &[Token],
    open: usize,
    close: usize,
    bindings: &Bindings,
) -> Option<String> {
    let aliases =
        super::bindings::attributes(tokens, open, close, vocabulary::REPOSITORY_NAME_ATTRIBUTE);
    match aliases.as_slice() {
        [alias] => bindings
            .string(&tokens[alias.clone()])
            .filter(|alias| !alias.is_empty()),
        [] => super::attribute_value(tokens, open, close, vocabulary::NAME_ATTRIBUTE, bindings)
            .filter(|name| !name.is_empty()),
        _ => None,
    }
}

pub(super) fn classify<'a>(label: &'a str, main: &Evidence) -> Repository<'a> {
    if let Some(body) = label.strip_prefix(CANONICAL_REPOSITORY_PREFIX) {
        return named(body).map_or(Repository::Uncertain, |_| Repository::External);
    }
    let Some(body) = label.strip_prefix(APPARENT_REPOSITORY_PREFIX) else {
        return Repository::Uncertain;
    };
    let Some((repository, absolute)) = named(body) else {
        return Repository::Uncertain;
    };
    match main {
        Evidence::Known(expected) if repository == expected => Repository::Main(absolute),
        Evidence::Known(_) => Repository::External,
        Evidence::Unknown => Repository::Uncertain,
    }
}

fn named(label: &str) -> Option<(&str, &str)> {
    let (repository, absolute) = label.split_once(ABSOLUTE_LABEL_PREFIX)?;
    (!repository.is_empty()).then_some((repository, absolute))
}
