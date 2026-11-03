use crate::constants::{
    BUILD_SYSTEM_PREFIX, FRONTEND_TERMS, FragmentedTerm, MatchStyle, RETIRED_NAME_PARTS,
};
use crate::source_policy::is_retired_frontend_file;
use std::path::Path;
use yolo_architecture_foundation::{
    SourceFile,
    model::Violation,
    text::{contains_ascii, contains_identifier, first_line_ascii, joined},
};

pub fn inspect(file: &SourceFile, violations: &mut Vec<Violation>) {
    if crate::web_vendor::is_vendored_web_asset(&file.absolute) {
        return;
    }
    if is_retired_frontend_file(&file.absolute) {
        violations.push(Violation::file(
            &file.relative,
            "JavaScript and TypeScript tooling is retired from the native repository",
        ));
    }
    let retired = joined(RETIRED_NAME_PARTS);
    if contains_ascii(&file.relative, &retired) {
        violations.push(Violation::file(
            &file.relative,
            "path contains the retired product identity",
        ));
    } else if let Some(line) = first_line_ascii(&file.text, &retired) {
        violations.push(Violation::line(
            &file.relative,
            line,
            "source contains the retired product identity",
        ));
    }

    for rule in FRONTEND_TERMS {
        let term = joined(rule.parts);
        if term_present(rule, &file.relative, &file.text, &file.absolute) {
            let line = first_line_ascii(&file.text, &term);
            let message = format!("disallowed frontend dependency or tool: {term}");
            match line {
                Some(line) => violations.push(Violation::line(&file.relative, line, message)),
                None => violations.push(Violation::file(&file.relative, message)),
            }
        }
    }
}

fn term_present(rule: &FragmentedTerm, relative: &str, text: &str, path: &Path) -> bool {
    let term = joined(rule.parts);
    match rule.style {
        MatchStyle::Substring => contains_ascii(relative, &term) || contains_ascii(text, &term),
        MatchStyle::Identifier => {
            contains_identifier(relative, &term) || contains_identifier(text, &term)
        }
        MatchStyle::Package => package_reference(relative, text, &term),
        MatchStyle::Runtime => runtime_reference(relative, text, path, &term),
    }
}

fn package_reference(relative: &str, text: &str, term: &str) -> bool {
    if contains_identifier(relative, term) {
        return true;
    }
    let markers = [
        format!("@{term}"),
        format!("\"{term}\":"),
        format!("'{term}':"),
        format!("/{term}/"),
    ];
    markers.iter().any(|marker| contains_ascii(text, marker))
}

fn runtime_reference(relative: &str, text: &str, path: &Path, term: &str) -> bool {
    let web_plugin = crate::source_policy::is_web_plugin_path(path)
        && path
            .extension()
            .is_some_and(|extension| matches!(extension.to_str(), Some("ts" | "tsx")));
    let allowed_builtins = (web_plugin && term == "node").then(|| text.replace("node:", ""));
    let text = allowed_builtins.as_deref().unwrap_or(text);
    let native_rust = path.extension().is_some_and(|extension| extension == "rs");
    if contains_identifier(relative, term)
        && (!native_rust || relative.starts_with(BUILD_SYSTEM_PREFIX))
    {
        return true;
    }
    let markers = [
        ["no", "de", "_", "modules"].concat(),
        ["no", "dejs"].concat(),
        ["rules_", "no", "de"].concat(),
        ["@types/", "no", "de"].concat(),
    ];
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    let frontend_metadata = [
        "bazel", "bzl", "cjs", "js", "json", "md", "mjs", "sh", "toml", "ts", "tsx", "yaml", "yml",
    ];
    if frontend_metadata.contains(&extension) {
        return markers.iter().any(|marker| contains_ascii(text, marker))
            || if extension == "md" {
                markdown_runtime_reference(text, term)
            } else {
                contains_identifier(text, term)
            };
    }
    extension == "rs" && rust_launches_runtime(text, term)
}

fn markdown_runtime_reference(text: &str, term: &str) -> bool {
    [
        format!("{term}.js"),
        format!("`{term}`"),
        format!("`{term} "),
        format!("{term} runtime"),
        format!("{term} package"),
    ]
    .iter()
    .any(|marker| contains_ascii(text, marker))
}

fn rust_launches_runtime(text: &str, term: &str) -> bool {
    let compact: String = text
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect();
    [
        format!("Command::new(\"{term}\")"),
        format!("process::Command::new(\"{term}\")"),
        format!("exec{term}"),
        format!("/usr/bin/env{term}"),
    ]
    .iter()
    .any(|marker| contains_ascii(&compact, marker))
}
