use std::path::Path;
use yolo_architecture_foundation::SourceFile;
use yolo_architecture_foundation::model::Violation;

const PROTOCOL_VERSION_MARKER: &str = "protocolversion";
const DECODED_VERSION_MARKER: &str = "decoded.version";
const VERSION_OFFSET_MARKER: &str = "versionoffset";
const RUST_CONSTANT_DECLARATION: &str = "const ";
const SWIFT_CONSTANT_DECLARATION: &str = "static let ";

pub fn inspect(file: &SourceFile, violations: &mut Vec<Violation>) {
    if !supported(&file.relative) {
        return;
    }
    for (index, line) in file.text.lines().enumerate() {
        let compact = compact_identifiers(line);
        if version_context(&compact)
            && numeric_literal(line)
            && !allowed_declaration(&file.relative, line, &compact)
        {
            violations.push(Violation::line(
                &file.relative,
                index + 1,
                "protocol versions and compatibility offsets must use centralized constants",
            ));
        }
    }
}

fn supported(relative: &str) -> bool {
    relative.ends_with(".rs") || relative.ends_with(".swift")
}

fn version_context(compact: &str) -> bool {
    compact.contains(PROTOCOL_VERSION_MARKER) || compact.contains(DECODED_VERSION_MARKER)
}

fn allowed_declaration(relative: &str, line: &str, compact: &str) -> bool {
    let declaration =
        line.contains(RUST_CONSTANT_DECLARATION) || line.contains(SWIFT_CONSTANT_DECLARATION);
    declaration && (centralized(relative) || compact.contains(VERSION_OFFSET_MARKER))
}

fn centralized(relative: &str) -> bool {
    let name = Path::new(relative)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    let lower = name.to_ascii_lowercase();
    lower == "constants.rs"
        || lower.ends_with("_constants.rs")
        || lower.ends_with("constants.swift")
        || relative == "core/src/model.rs"
}

fn compact_identifiers(line: &str) -> String {
    line.chars()
        .filter(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '.' | ':' | '+')
        })
        .flat_map(char::to_lowercase)
        .collect()
}

fn numeric_literal(line: &str) -> bool {
    let bytes = line.as_bytes();
    let mut quoted = false;
    let mut escaped = false;
    for (index, byte) in bytes.iter().copied().enumerate() {
        if byte == b'"' && !escaped {
            quoted = !quoted;
        }
        if !quoted && byte.is_ascii_digit() {
            let before = index.checked_sub(1).map(|position| bytes[position]);
            if !before.is_some_and(identifier_byte) {
                return true;
            }
        }
        escaped = byte == b'\\' && !escaped;
        if byte != b'\\' {
            escaped = false;
        }
    }
    false
}

fn identifier_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}
