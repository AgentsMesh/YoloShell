use yolo_architecture_foundation::{SourceFile, model::Violation};
use yolo_architecture_syntax::targeted_tokens::tokenize;

const CORE_ROOT: &str = "core/crates/";
const RUST_SOURCE: &str = "/src/";
const RETIRED_CONTRACTS: &[&str] = &[
    "LEGACY_PROTOCOL_VERSION",
    "LEGACY_CUSTODY_RECORD_SCHEMA_VERSION",
    "LEGACY_CREATION_RECEIPT_SCHEMA_VERSION",
    "STARTUP_CHECKPOINT_LEGACY_SCHEMA_VERSION",
    "STARTUP_CHECKPOINT_RUNTIME_AUTHORITY_SCHEMA_VERSION",
    "LegacyRecordWire",
    "WorkerAuthWire",
    "WorkerAuthResult",
    "BootIdentityAliases",
    "relation_to_macos_alias",
    "LegacyV3",
    "LegacyV3Unspecified",
    "uses_legacy_protocol",
    "is_legacy_schema",
    "migrate_legacy_retirement_with_evidence",
    stringify!(legacy_origins),
    "legacyHostSchemaVersion",
    "HubWorkspaceMigration",
    "HubOriginMigration",
];
const MESSAGE: &str =
    "unreleased product must use current contracts without retired schema or protocol adapters";

pub(super) fn inspect(file: &SourceFile, violations: &mut Vec<Violation>) {
    if !(file.relative.starts_with(CORE_ROOT) && file.relative.contains(RUST_SOURCE)) {
        return;
    }
    for token in tokenize(&file.text) {
        if RETIRED_CONTRACTS.iter().any(|name| token.identifier(name)) {
            violations.push(Violation::line(&file.relative, token.line, MESSAGE));
        }
    }
}

#[cfg(test)]
#[path = "../unit_tests/current_product.rs"]
mod tests;
