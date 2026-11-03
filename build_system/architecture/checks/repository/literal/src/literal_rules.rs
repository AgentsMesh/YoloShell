use crate::constants::{
    CLI_PREFIX, MIN_SEMANTIC_LITERAL_LENGTH, PROTOCOL_CONTEXT_LOOKBACK, REQUIRED_CENTRAL_LITERALS,
};
use std::collections::BTreeSet;
use std::path::Path;
use yolo_architecture_foundation::SourceFile;
use yolo_architecture_foundation::model::Violation;
use yolo_architecture_foundation::text::{line_at, protocol_shaped, quoted_literals};

#[path = "literal_rules/operational.rs"]
mod operational;
#[path = "literal_rules/serde.rs"]
mod serde;

pub fn registry(files: &[SourceFile]) -> BTreeSet<String> {
    let mut values: BTreeSet<String> = files
        .iter()
        .filter(|file| centralized(&file.relative))
        .flat_map(|file| {
            quoted_literals(&file.text)
                .into_iter()
                .filter(|literal| registry_candidate(file, literal.line, &literal.value))
                .map(|literal| literal.value)
        })
        .collect();
    values.extend(
        REQUIRED_CENTRAL_LITERALS
            .iter()
            .map(|value| (*value).to_owned()),
    );
    values
}

pub fn inspect(file: &SourceFile, known: &BTreeSet<String>, violations: &mut Vec<Violation>) {
    operational::inspect(file, violations);
    if centralized(&file.relative)
        || !(file.relative.ends_with(".rs") || file.relative.ends_with(".swift"))
    {
        return;
    }
    for literal in quoted_literals(&file.text) {
        match serde::metadata_disposition(
            &file.text,
            literal.line,
            &literal.value,
            known.contains(&literal.value),
        ) {
            serde::MetadataDisposition::Exempt => continue,
            serde::MetadataDisposition::UnregisteredTag => {
                violations.push(Violation::line(
                    &file.relative,
                    literal.line,
                    format!(
                        "serde protocol tag {:?} must be declared centrally",
                        literal.value
                    ),
                ));
                continue;
            }
            serde::MetadataDisposition::Ordinary => {}
        }
        let known_in_context = known.contains(&literal.value)
            && (protocol_shaped(&literal.value)
                || semantic_context(&file.text, literal.line)
                || cli_context(&file.text, literal.line));
        if literal.value.starts_with(CLI_PREFIX) || known_in_context {
            violations.push(Violation::line(
                &file.relative,
                literal.line,
                format!(
                    "CLI or protocol literal {:?} must use a centralized constant",
                    literal.value
                ),
            ));
            continue;
        }
        if protocol_shaped(&literal.value) && semantic_context(&file.text, literal.line) {
            violations.push(Violation::line(
                &file.relative,
                literal.line,
                format!(
                    "protocol literal {:?} must be declared centrally",
                    literal.value
                ),
            ));
        }
    }
}

fn cli_context(text: &str, line: usize) -> bool {
    let start = line.saturating_sub(PROTOCOL_CONTEXT_LOOKBACK + 1);
    let context = (start..=line)
        .map(|number| line_at(text, number))
        .collect::<Vec<_>>()
        .join(" ");
    [
        "try_parse_from",
        "Command::new",
        ".arg(",
        ".args(",
        "arguments:",
        "let arguments",
        "let args",
    ]
    .iter()
    .any(|marker| context.contains(marker))
}

fn centralized(relative: &str) -> bool {
    if relative.starts_with("build_system/architecture/contracts/src/")
        || relative.starts_with("build_system/release/contracts/src/")
        || relative.starts_with("build_system/coverage/protocol/src/")
    {
        return true;
    }
    let path = Path::new(relative);
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    let lower = name.to_ascii_lowercase();
    let product_source = (relative.starts_with("core/src/")
        || relative.starts_with("core/crates/")
        || (relative.starts_with("plugins/") && path.extension().is_some_and(|ext| ext == "rs")))
        || relative.starts_with("plugins/macos/Sources/")
        || relative.starts_with("build_system/");
    (product_source
        && (lower == "constants.rs"
            || lower.ends_with("_constants.rs")
            || lower == "protocol.rs"
            || lower.ends_with("constants.swift")
            || lower.ends_with("protocol.swift")))
        || (relative.starts_with("integration_tests/") && lower == "constants.rs")
        || ([
            "plugins/macos/Tests/",
            "plugins/macos/unit_tests/",
            "plugins/macos/integration_tests/",
            "plugins/macos/test_support/",
        ]
        .iter()
        .any(|root| relative.starts_with(root))
            && lower.ends_with("constants.swift"))
}

fn registry_candidate(file: &SourceFile, line: usize, value: &str) -> bool {
    if value.starts_with(CLI_PREFIX) || protocol_shaped(value) {
        return true;
    }
    if value.len() < MIN_SEMANTIC_LITERAL_LENGTH
        || !value.bytes().all(|byte| byte.is_ascii_lowercase())
    {
        return false;
    }
    let context = line_at(&file.text, line);
    ["COMMAND", "ARGUMENT", "EXECUTABLE", "PROGRAM", "_TAG"]
        .iter()
        .any(|marker| context.contains(marker))
        || swift_cli_scope(file, line)
}

fn swift_cli_scope(file: &SourceFile, line: usize) -> bool {
    if !file.relative.ends_with(".swift") {
        return false;
    }
    for number in (1..=line).rev() {
        let candidate = line_at(&file.text, number);
        if candidate.contains("enum ") {
            return candidate.contains("Command") || candidate.contains("Argument");
        }
        if candidate.contains('}') {
            return false;
        }
    }
    false
}

fn semantic_context(text: &str, line: usize) -> bool {
    let start = line.saturating_sub(PROTOCOL_CONTEXT_LOOKBACK);
    let context = (start..=line)
        .map(|number| line_at(text, number))
        .collect::<Vec<_>>()
        .join(" ");
    let constructor = [
        "ErrorBody::new",
        "EventEnvelope::new",
        "RequestEnvelope::new",
        "ResponseEnvelope::new",
    ]
    .iter()
    .any(|marker| context.contains(marker));
    let current = line_at(text, line);
    constructor || current.contains("const ") || current.contains("static let ")
}
