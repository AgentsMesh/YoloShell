use crate::constants::{
    PROTOCOL_CONTEXT_LOOKBACK, SERDE_ATTRIBUTE_CLOSE, SERDE_ATTRIBUTE_OPEN, SERDE_BYTES_ADAPTER,
    SERDE_RENAME_ALL_KEY, SERDE_TAG_KEY, SERDE_WITH_KEY,
};
use yolo_architecture_foundation::text::line_at;

#[cfg(test)]
#[path = "../../unit_tests/serde_decision_tests.rs"]
mod tests;

pub(super) enum MetadataDisposition {
    Ordinary,
    Exempt,
    UnregisteredTag,
}

pub(super) fn metadata_disposition(
    text: &str,
    line: usize,
    value: &str,
    centralized: bool,
) -> MetadataDisposition {
    let Some(attribute) = attribute_at(text, line) else {
        return MetadataDisposition::Ordinary;
    };
    if assigned(&attribute, SERDE_TAG_KEY, value) {
        return if centralized {
            MetadataDisposition::Exempt
        } else {
            MetadataDisposition::UnregisteredTag
        };
    }
    if assigned(&attribute, SERDE_RENAME_ALL_KEY, value)
        || (value == SERDE_BYTES_ADAPTER && assigned(&attribute, SERDE_WITH_KEY, value))
    {
        return MetadataDisposition::Exempt;
    }
    MetadataDisposition::Ordinary
}

fn attribute_at(text: &str, line: usize) -> Option<String> {
    let current = line_at(text, line);
    if let Some(start) = current.rfind(SERDE_ATTRIBUTE_OPEN) {
        return Some(current[start..].to_owned());
    }
    let start = line.saturating_sub(PROTOCOL_CONTEXT_LOOKBACK);
    let preceding = (start..line)
        .map(|number| line_at(text, number))
        .collect::<Vec<_>>()
        .join(" ");
    let attribute_start = preceding.rfind(SERDE_ATTRIBUTE_OPEN)?;
    let attribute = &preceding[attribute_start..];
    if attribute.contains(SERDE_ATTRIBUTE_CLOSE) {
        return None;
    }
    Some(format!("{attribute} {current}"))
}

fn assigned(attribute: &str, key: &str, value: &str) -> bool {
    let compact = attribute
        .chars()
        .filter(|character| !character.is_ascii_whitespace())
        .collect::<String>();
    compact.contains(&format!("{key}=\"{value}\""))
}
