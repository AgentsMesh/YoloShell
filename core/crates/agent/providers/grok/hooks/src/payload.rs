use std::path::PathBuf;

/// Grok sends its camelCase envelope and the same values again under Claude's
/// snake_case keys, so fields are read by either name instead of deserialized
/// into one struct (serde would report duplicate fields).
pub(super) struct HookPayload<'a> {
    value: &'a serde_json::Value,
}

impl<'a> HookPayload<'a> {
    pub(super) fn new(value: &'a serde_json::Value) -> Self {
        Self { value }
    }

    pub(super) fn text(&self, camel: &str, snake: &str) -> Option<&'a str> {
        self.value
            .get(camel)
            .or_else(|| self.value.get(snake))
            .and_then(serde_json::Value::as_str)
    }

    pub(super) fn path(&self, camel: &str, snake: &str) -> Option<PathBuf> {
        self.text(camel, snake).map(PathBuf::from)
    }

    pub(super) fn flag(&self, camel: &str, snake: &str) -> Option<bool> {
        self.value
            .get(camel)
            .or_else(|| self.value.get(snake))
            .and_then(serde_json::Value::as_bool)
    }
}

/// Accept only the canonical hyphenated form of a UUID session id, so the
/// derived session directory and the provider's own directory agree.
pub(super) fn normalized_session_id(value: &str) -> Option<String> {
    let parsed = uuid::Uuid::parse_str(value).ok()?;
    let canonical = parsed.hyphenated().to_string();
    (canonical == value).then_some(canonical)
}
