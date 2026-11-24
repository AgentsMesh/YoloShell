use super::test_constants::{
    ARTIFACT_SHA256_JSON_KEY, BAZEL_RUNFILES_ROOT_DETAIL, BAZEL_WORKSPACE_NAME_DETAIL,
    NORMALIZED_WHITESPACE_SEPARATOR, SOURCE_REVISION_JSON_KEY, TEST_SRCDIR_ENVIRONMENT,
    TEST_WORKSPACE_ENVIRONMENT,
};
use crate::constants::*;
use std::path::PathBuf;

const DOCUMENT_PATH: &str = "docs/acceptance-receipt.md";
const DOCUMENT_READ_DETAIL: &str = "acceptance receipt operator contract";
const REQUIRED_SCHEMA_FIELDS: &[&str] = &[
    "payload",
    "signature",
    SOURCE_REVISION_JSON_KEY,
    ARTIFACT_SHA256_JSON_KEY,
    "provenance_sha256",
    "checklist_schema",
    "operator_identity",
    "completed_at_unix_seconds",
    "expires_at_unix_seconds",
];
const REQUIRED_METADATA_CONTRACTS: &[&str] = &[
    "permission mode exactly `0400`",
    "exactly one hard link",
    "no extended ACL",
    "owned by its effective user",
];

#[test]
fn operator_contract_tracks_the_verified_schema_and_targets() {
    let document = document();
    for value in REQUIRED_SCHEMA_FIELDS
        .iter()
        .copied()
        .chain([
            ACCEPTANCE_AUTHORITY,
            ACCEPTANCE_ALGORITHM,
            ACCEPTANCE_PRODUCT,
            ACCEPTANCE_CHECKLIST_SCHEMA,
            CLAUDE_ACCEPTANCE_VERSION_OUTPUT,
            CODEX_ACCEPTANCE_VERSION_OUTPUT,
        ])
        .chain(REQUIRED_ACCEPTANCE_CHECKS.iter().copied())
    {
        assert!(document.contains(value), "missing contract value: {value}");
    }
}

#[test]
fn operator_contract_states_every_descriptor_metadata_requirement() {
    let document = document();
    for requirement in REQUIRED_METADATA_CONTRACTS {
        assert!(document.contains(requirement), "missing: {requirement}");
    }
}

fn document() -> String {
    let source_root = std::env::var_os(TEST_SRCDIR_ENVIRONMENT).expect(BAZEL_RUNFILES_ROOT_DETAIL);
    let workspace =
        std::env::var_os(TEST_WORKSPACE_ENVIRONMENT).expect(BAZEL_WORKSPACE_NAME_DETAIL);
    std::fs::read_to_string(
        PathBuf::from(source_root)
            .join(workspace)
            .join(DOCUMENT_PATH),
    )
    .expect(DOCUMENT_READ_DETAIL)
    .split_whitespace()
    .collect::<Vec<_>>()
    .join(NORMALIZED_WHITESPACE_SEPARATOR)
}
