use std::collections::BTreeMap;
use yolo_coverage_protocol::{
    FUNCTION_DATA_TAG, FUNCTION_DEFINITION_TAG, FunctionToken, TOKEN_ROOT, parse_token,
};

const EMPTY_PLACEHOLDER_LINES: [&str; 4] = ["FNF:0", "FNH:0", "LF:0", "LH:0"];

#[derive(Debug)]
pub struct FunctionEntry {
    pub count: u64,
    pub line: u64,
    pub name: String,
}

pub fn synthetic(lines: &[&str]) -> Result<Vec<FunctionEntry>, String> {
    let mut definitions = BTreeMap::<String, u64>::new();
    let mut data = BTreeMap::<String, u64>::new();
    for line in lines {
        if let Some(value) = line.strip_prefix(FUNCTION_DEFINITION_TAG) {
            let (raw_line, name) = value
                .split_once(',')
                .ok_or_else(|| format!("malformed FN entry {value:?}"))?;
            if parse_token(name)?.is_some() {
                let line = number(raw_line, "function line")?;
                if definitions.insert(name.to_owned(), line).is_some() {
                    return Err(format!("duplicate synthetic FN entry {name:?}"));
                }
            }
        } else if let Some(value) = line.strip_prefix(FUNCTION_DATA_TAG) {
            let (raw_count, name) = value
                .split_once(',')
                .ok_or_else(|| format!("malformed FNDA entry {value:?}"))?;
            if parse_token(name)?.is_some() {
                let count = number(raw_count, "function count")?;
                if data.insert(name.to_owned(), count).is_some() {
                    return Err(format!("duplicate synthetic FNDA entry {name:?}"));
                }
            }
        } else if line.contains(TOKEN_ROOT) {
            return Err(format!(
                "synthetic token appears in an invalid LCOV line {line:?}"
            ));
        }
    }
    if definitions.keys().ne(data.keys()) {
        return Err("synthetic function definitions and counts differ".to_owned());
    }
    definitions
        .into_iter()
        .map(|(name, line)| {
            Ok(FunctionEntry {
                count: data[&name],
                line,
                name,
            })
        })
        .collect()
}

pub fn empty_placeholder(lines: &[&str]) -> bool {
    lines.len() == EMPTY_PLACEHOLDER_LINES.len()
        && EMPTY_PLACEHOLDER_LINES
            .iter()
            .all(|expected| lines.contains(expected))
}

pub fn validate_location(entry: &FunctionEntry, token: &FunctionToken<'_>) -> Result<(), String> {
    let expected = match token {
        FunctionToken::Marker
        | FunctionToken::SourceInventory { .. }
        | FunctionToken::InstanceInventory { .. }
        | FunctionToken::InstanceMember { .. } => yolo_coverage_protocol::MINIMUM_SOURCE_POSITION,
        FunctionToken::Header { line, .. } | FunctionToken::Ordinal { line, .. } => *line,
    };
    if entry.line != expected {
        return Err(format!(
            "synthetic function location {} differs from token location {expected}",
            entry.line
        ));
    }
    Ok(())
}

fn number(value: &str, label: &str) -> Result<u64, String> {
    value
        .parse::<u64>()
        .map_err(|_| format!("invalid synthetic {label} {value:?}"))
}
