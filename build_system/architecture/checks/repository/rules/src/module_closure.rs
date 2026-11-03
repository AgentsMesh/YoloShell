use std::collections::{BTreeMap, BTreeSet};
use yolo_architecture_foundation::{SourceFile, model::Violation};
use yolo_architecture_syntax::targeted_tokens::{Kind, Token, tokenize};

const DIRECT_LINE_LIMIT: usize = 1_200;
const POLICY_MESSAGE: &str =
    "module scope exceeds the direct line budget; split it by responsibility";
const RUST_SUFFIX: &str = ".rs";

pub fn inspect(files: &[SourceFile], violations: &mut Vec<Violation>) {
    let sources = files
        .iter()
        .map(|file| (file.relative.as_str(), file))
        .collect::<BTreeMap<_, _>>();
    for file in files.iter().filter(|file| production(&file.relative)) {
        let total = file.text.lines().count() + included_lines(file, &sources);
        if total <= DIRECT_LINE_LIMIT {
            continue;
        }
        let recorded = baseline(&file.relative);
        if recorded.is_some_and(|baseline| total <= baseline) {
            continue;
        }
        let detail = recorded.map_or_else(
            || format!("{total} lines, limit {DIRECT_LINE_LIMIT}"),
            |baseline| format!("{total} lines, baseline {baseline}, limit {DIRECT_LINE_LIMIT}"),
        );
        violations.push(Violation::file(
            &file.relative,
            format!("{POLICY_MESSAGE}: {detail}"),
        ));
    }
}

fn included_lines(file: &SourceFile, sources: &BTreeMap<&str, &SourceFile>) -> usize {
    let mut visited = BTreeSet::new();
    collect_included(file.relative.as_str(), &file.text, sources, &mut visited)
}

fn collect_included(
    source: &str,
    text: &str,
    sources: &BTreeMap<&str, &SourceFile>,
    visited: &mut BTreeSet<String>,
) -> usize {
    let mut total = 0;
    for target in include_targets(text) {
        let path = resolve(source, &target);
        if !visited.insert(path.clone()) {
            continue;
        }
        if let Some(included) = sources.get(path.as_str()) {
            total += included.text.lines().count();
            total += collect_included(&included.relative, &included.text, sources, visited);
        }
    }
    total
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum IncludeSite {
    Literal(String),
    Dynamic,
}

pub(crate) fn include_sites(text: &str) -> Vec<IncludeSite> {
    let tokens = tokenize(text);
    (0..tokens.len())
        .filter_map(|index| include_site(&tokens, index))
        .collect()
}

fn include_targets(text: &str) -> Vec<String> {
    include_sites(text)
        .into_iter()
        .filter_map(|site| match site {
            IncludeSite::Literal(path) => Some(path),
            IncludeSite::Dynamic => None,
        })
        .collect()
}

fn include_site(tokens: &[Token], index: usize) -> Option<IncludeSite> {
    if !tokens.get(index)?.identifier("include")
        || !tokens.get(index + 1)?.symbol("!")
        || !opens_arguments(tokens.get(index + 2)?)
    {
        return None;
    }
    match &tokens.get(index + 3)?.kind {
        Kind::String(path) => Some(IncludeSite::Literal(path.clone())),
        _ => Some(IncludeSite::Dynamic),
    }
}

fn opens_arguments(token: &Token) -> bool {
    ["(", "[", "{"]
        .into_iter()
        .any(|symbol| token.symbol(symbol))
}

pub(crate) fn resolve(source: &str, target: &str) -> String {
    let directory = source
        .rsplit_once('/')
        .map_or("", |(directory, _)| directory);
    let mut components = Vec::new();
    for part in directory
        .split('/')
        .chain(target.split('/'))
        .filter(|part| !part.is_empty())
    {
        match part {
            "." => {}
            ".." => {
                components.pop();
            }
            value => components.push(value),
        }
    }
    let resolved = components.join("/");
    if resolved.ends_with(RUST_SUFFIX) {
        resolved
    } else {
        format!("{resolved}{RUST_SUFFIX}")
    }
}

pub(crate) fn production(path: &str) -> bool {
    yolo_architecture_foundation::source_path::is_production_rust_source(path)
}

fn baseline(path: &str) -> Option<usize> {
    BASELINE
        .iter()
        .find(|(entry, _)| *entry == path)
        .map(|(_, limit)| *limit)
}

const BASELINE: &[(&str, usize)] = &[
    ("core/crates/runtime/engine/host/src/lifecycle/mod.rs", 2_399),
    ("core/crates/runtime/engine/worker/src/pty/mod.rs", 1_225),
];
