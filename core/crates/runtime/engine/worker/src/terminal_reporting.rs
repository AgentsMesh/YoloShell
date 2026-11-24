use super::supervision;

pub(super) fn discard_error(error: anyhow::Error) {
    let _ = error_text(error);
}

pub(super) fn error_text(error: anyhow::Error) -> String {
    supervision::error_detail(error)
}

pub(super) fn error_ref_text(error: &anyhow::Error) -> String {
    supervision::contain(|| format!("{error:#}"))
        .unwrap_or_else(|| supervision::error_ref_detail(error))
}

pub(super) fn warn_error(error: anyhow::Error, message: &'static str) {
    let detail = error_text(error);
    warn_detail(&detail, message);
}

pub(super) fn warn_detail(detail: &str, message: &'static str) {
    let _ = supervision::contain(|| tracing::warn!(%detail, message = message));
}

pub(super) fn warn_message(message: &'static str) {
    let _ = supervision::contain(|| tracing::warn!(message));
}

pub(super) fn error_detail(detail: &str, message: &'static str) {
    let _ = supervision::contain(|| tracing::error!(%detail, message = message));
}

pub(super) fn error_error(error: anyhow::Error, message: &'static str) {
    let detail = error_text(error);
    error_detail(&detail, message);
}

pub(super) fn error_message(message: &'static str) {
    let _ = supervision::contain(|| tracing::error!(message));
}

pub(super) fn debug_message(message: &'static str) {
    let _ = supervision::contain(|| tracing::debug!(message));
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[cfg(any(feature = "yolo-integration-tests", feature = "yolo-worker-unit-terminal"))]
#[path = "../unit_tests/terminal/terminal_reporting_tests.rs"]
mod tests;
