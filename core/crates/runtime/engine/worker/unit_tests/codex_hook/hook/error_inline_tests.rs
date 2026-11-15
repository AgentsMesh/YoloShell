use super::*;

const STORAGE_DETAIL: &str = "storage detail";

#[cfg(feature = "yolo-unit-tests")]
#[test]
fn storage_rejection_uses_storage_diagnostic_context() {
    let rejection = CodexHookRejection::Storage(anyhow::anyhow!(STORAGE_DETAIL));
    let rendered = rejection.to_string();

    assert!(rendered.starts_with(CODEX_STORAGE_ERROR));
    assert!(rendered.contains(STORAGE_DETAIL));
}
