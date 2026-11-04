use anyhow::Result;
use rusqlite::ffi::ErrorCode;

#[derive(Debug, thiserror::Error)]
pub(super) enum RebuildableCatalogError {
    #[error("unsafe catalog artifact: {0}")]
    UnsafeArtifact(String),
    #[error("unsupported catalog schema: {0}")]
    Schema(String),
    #[error("invalid catalog content: {0}")]
    Content(String),
    #[error("catalog integrity check failed: {0}")]
    Integrity(String),
}

pub(super) fn unsafe_artifact(message: impl Into<String>) -> anyhow::Error {
    RebuildableCatalogError::UnsafeArtifact(message.into()).into()
}

pub(super) fn schema(message: impl Into<String>) -> anyhow::Error {
    RebuildableCatalogError::Schema(message.into()).into()
}

pub(super) fn content(message: impl Into<String>) -> anyhow::Error {
    RebuildableCatalogError::Content(message.into()).into()
}

pub(super) fn integrity(message: impl Into<String>) -> anyhow::Error {
    RebuildableCatalogError::Integrity(message.into()).into()
}

pub(super) fn schema_result<T>(result: rusqlite::Result<T>) -> Result<T> {
    if let Some(message) = super::secure_vfs::take_violation() {
        return Err(unsafe_artifact(message));
    }
    result.map_err(|error| {
        if explicit_schema_error(&error) {
            schema(error.to_string())
        } else {
            error.into()
        }
    })
}

pub(super) fn is_rebuildable(error: &anyhow::Error) -> bool {
    error.chain().any(|cause| {
        cause.is::<RebuildableCatalogError>()
            || cause
                .downcast_ref::<rusqlite::Error>()
                .is_some_and(corrupt_sqlite_error)
    })
}

fn corrupt_sqlite_error(error: &rusqlite::Error) -> bool {
    error.sqlite_error().is_some_and(|error| {
        matches!(
            error.code,
            ErrorCode::DatabaseCorrupt | ErrorCode::NotADatabase | ErrorCode::SchemaChanged
        )
    })
}

fn explicit_schema_error(error: &rusqlite::Error) -> bool {
    if error.sqlite_error().is_some_and(|error| {
        matches!(
            error.code,
            ErrorCode::SchemaChanged
                | ErrorCode::ConstraintViolation
                | ErrorCode::TypeMismatch
                | ErrorCode::Unknown
        )
    }) {
        return true;
    }
    matches!(
        error,
        rusqlite::Error::QueryReturnedNoRows
            | rusqlite::Error::QueryReturnedMoreThanOneRow
            | rusqlite::Error::InvalidColumnIndex(_)
            | rusqlite::Error::InvalidColumnName(_)
            | rusqlite::Error::InvalidColumnType(_, _, _)
    )
}

#[cfg(test)]
#[cfg(feature = "yolo-unit-tests")]
#[path = "../unit_tests/rebuild_reason_inline_tests.rs"]
mod tests;
