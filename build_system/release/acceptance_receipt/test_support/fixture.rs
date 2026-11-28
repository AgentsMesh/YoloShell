use crate::constants::{ACCEPTANCE_RECORD_SCHEMA, CLAUDE_PROVIDER, REQUIRED_ACCEPTANCE_CHECKS};
use crate::{
    AcceptanceCandidateBinding, AcceptanceCheckResult, AcceptanceExpectation, AcceptancePlatform,
    AcceptanceProviderVersion, AcceptanceRecord,
};
use crate::{
    AcceptanceReceipt, from_bytes_with_test_key, open_with_test_key, signed_with_test_authority,
    test_public_key,
};
use yolo_release_foundation::digest::bytes_sha256;
pub const NOW_SECONDS: u64 = 1_800_000_000;
pub const EXPIRY_SECONDS: u64 = NOW_SECONDS + 3_600;
pub const CHECKLIST_SCHEMA: &str = "yolo-visible-ui-v2";
pub const OPERATOR_IDENTITY: &str = "operator@example.test";
pub const LEGACY_CODEX_OUTPUT_PREFIX: &str = "codex ";
pub const LEGACY_CLAUDE_OUTPUT_PREFIX: &str = "claude-code ";
pub const PROVIDER_VERSION_SUFFIX: &str = "-beta";
pub const PROVIDER_LEADING_SPACE: &str = " ";
pub const PROVIDER_LINE_FEED: &str = "\n";
pub const PROVIDER_CRLF: &str = "\r\n";
pub const CLAUDE_PROVIDER_INDEX: usize = 0;
pub const CODEX_PROVIDER_INDEX: usize = 1;

pub fn record() -> AcceptanceRecord {
    AcceptanceRecord {
        schema: ACCEPTANCE_RECORD_SCHEMA,
        product: "Yolo".to_owned(),
        source_revision: "0123456789abcdef0123456789abcdef01234567".to_owned(),
        artifact_sha256: digest('a'),
        provenance_sha256: digest('b'),
        checklist_schema: CHECKLIST_SCHEMA.to_owned(),
        providers: vec![
            AcceptanceProviderVersion::new(
                CLAUDE_PROVIDER,
                crate::constants::CLAUDE_ACCEPTANCE_VERSION_OUTPUT,
            )
            .unwrap(),
            AcceptanceProviderVersion::new(
                crate::constants::CODEX_PROVIDER,
                crate::constants::CODEX_ACCEPTANCE_VERSION_OUTPUT,
            )
            .unwrap(),
        ],
        desktop_os: AcceptancePlatform::new("macOS", "14.2", "arm64").unwrap(),
        remote_os: AcceptancePlatform::new("macOS", "14.2", "arm64").unwrap(),
        operator_identity: OPERATOR_IDENTITY.to_owned(),
        completed_at_unix_seconds: NOW_SECONDS - 60,
        expires_at_unix_seconds: EXPIRY_SECONDS,
        results: REQUIRED_ACCEPTANCE_CHECKS
            .iter()
            .map(|identifier| AcceptanceCheckResult::passed(*identifier).unwrap())
            .collect(),
    }
}

pub fn bytes() -> Vec<u8> {
    signed_with_test_authority(record()).unwrap()
}

pub fn try_signed(record: AcceptanceRecord) -> anyhow::Result<Vec<u8>> {
    signed_with_test_authority(record)
}

pub fn parse(bytes: &[u8]) -> anyhow::Result<AcceptanceReceipt> {
    from_bytes_with_test_key(bytes, test_public_key())
}

pub fn open(path: &std::path::Path) -> anyhow::Result<AcceptanceReceipt> {
    open_with_test_key(path, test_public_key())
}

pub fn expectation() -> AcceptanceExpectation {
    AcceptanceExpectation::new(
        AcceptanceCandidateBinding::new(
            record().source_revision,
            record().artifact_sha256,
            record().provenance_sha256,
        )
        .unwrap(),
        CHECKLIST_SCHEMA,
        record().providers,
        record().desktop_os,
        record().remote_os,
        OPERATOR_IDENTITY,
        NOW_SECONDS,
    )
    .unwrap()
}

pub fn digest(character: char) -> String {
    bytes_sha256(character.to_string().as_bytes())
}

pub fn replace_once(bytes: &[u8], before: &str, after: &str) -> Vec<u8> {
    let text = String::from_utf8(bytes.to_vec()).unwrap();
    let replaced = text.replacen(before, after, 1);
    replaced.into_bytes()
}
