pub const ACCEPTANCE_RECORD_SCHEMA: u16 = 1;
pub const ACCEPTANCE_ENVELOPE_SCHEMA: u16 = 1;
pub const ACCEPTANCE_AUTHORITY: &str = "yolo-release-acceptance-v1";
pub const ACCEPTANCE_ALGORITHM: &str = "ed25519";
pub const ACCEPTANCE_PRODUCT: &str = "Yolo";
pub const ACCEPTANCE_CHECK_STATUS_PASSED: &str = "passed";
pub const ACCEPTANCE_PLATFORM_NAME_MAX_BYTES: usize = 128;
pub const ACCEPTANCE_PLATFORM_NAME: &str = "macOS";
pub const ACCEPTANCE_PLATFORM_ARCHITECTURE: &str = "arm64";
pub const MIN_MACOS_MAJOR_VERSION: u16 = 14;
pub const MIN_MACOS_MINOR_VERSION: u16 = 2;
pub const MIN_MACOS_VERSION_COMPONENTS: usize = 2;
pub const MAX_MACOS_VERSION_COMPONENTS: usize = 3;
pub const CODEX_PROVIDER: &str = "codex";
#[cfg(test)]
pub const CODEX_ACCEPTANCE_VERSION: &str = "0.150.1";
pub const CODEX_ACCEPTANCE_VERSION_OUTPUT: &str = "codex-cli 0.150.1";
pub const CLAUDE_PROVIDER: &str = "claude_code";
#[cfg(test)]
pub const CLAUDE_ACCEPTANCE_VERSION: &str = "2.1.241";
pub const CLAUDE_ACCEPTANCE_VERSION_OUTPUT: &str = "2.1.241 (Claude Code)";
pub const REQUIRED_ACCEPTANCE_PROVIDERS: &[&str] = &[CLAUDE_PROVIDER, CODEX_PROVIDER];
pub const PENDING_STATUS: &str = "pending";
pub const ACCEPTANCE_CHECKLIST_SCHEMA: &str = "yolo-visible-ui-v2";
pub const ACCEPTANCE_MAX_VALIDITY_SECONDS: u64 = 31 * 24 * 60 * 60;

pub const REQUIRED_ACCEPTANCE_CHECKS: &[&str] = &[
    NORMAL_VISIBLE_LAUNCH_CHECK,
    TITLE_BAR_DRAGGING_CHECK,
    SPLIT_CONTROLS_CHECK,
    TERMINAL_INPUT_AND_RENDERING_CHECK,
    REAL_CODEX_RECOVERY_CHECK,
    REAL_CLAUDE_RECOVERY_CHECK,
    REMOTE_MACOS_RECOVERY_CHECK,
];

pub const NORMAL_VISIBLE_LAUNCH_CHECK: &str = "normal_visible_launch";
pub const TITLE_BAR_DRAGGING_CHECK: &str = "title_bar_dragging";
pub const SPLIT_CONTROLS_CHECK: &str = "split_controls";
pub const TERMINAL_INPUT_AND_RENDERING_CHECK: &str = "terminal_input_and_rendering";
pub const REAL_CODEX_RECOVERY_CHECK: &str = "real_codex_detection_status_and_recovery";
pub const REAL_CLAUDE_RECOVERY_CHECK: &str = "real_claude_detection_status_and_recovery";
pub const REMOTE_MACOS_RECOVERY_CHECK: &str = "remote_macos_multiplexed_recovery";

pub const ED25519_PUBLIC_KEY_BYTES: usize = 32;
pub const ED25519_SIGNATURE_BYTES: usize = 64;
pub const HEX_CHARACTERS_PER_BYTE: usize = 2;
pub const SHA256_HEX_CHARACTERS: usize = 64;
pub const SIGNATURE_HEX_CHARACTERS: usize = ED25519_SIGNATURE_BYTES * HEX_CHARACTERS_PER_BYTE;

pub const MAX_ACCEPTANCE_RECEIPT_BYTES: u64 = 64 * 1024;
pub const MAX_ACCEPTANCE_FIELD_BYTES: usize = 512;
pub const MAX_ACCEPTANCE_PROVIDER_ENTRIES: usize = 16;
pub const EXPECTED_FILE_LINKS: u64 = 1;
pub const PERMISSION_BITS_MASK: u32 = 0o777;
pub const READ_ONLY_FILE_MODE: u32 = 0o400;
pub const CLOCK_ERROR: &str = "system clock precedes the Unix epoch";

// This is a repository-pinned public key. Its private counterpart lives only
// in the offline acceptance signing environment and never enters this tree.
pub const PINNED_ACCEPTANCE_PUBLIC_KEY: [u8; ED25519_PUBLIC_KEY_BYTES] = [
    0x04, 0xa9, 0x24, 0x0a, 0x1a, 0xe6, 0x04, 0x4e, 0xa4, 0xc6, 0x13, 0x5a, 0x7e, 0x5c, 0xb0, 0x46,
    0x87, 0x1a, 0xcd, 0x5a, 0xac, 0x66, 0x35, 0x5e, 0x60, 0x83, 0x99, 0xa8, 0x49, 0xa9, 0xf9, 0xa6,
];
pub const PINNED_ACCEPTANCE_KEY_FINGERPRINT: &str =
    "f3ced7ab2eeb81f164a2d61464902ddfdeb958b60ee9ec6c3b950ca4b7e541dc";

pub const RECEIPT_FILE_ERROR: &str = "read signed acceptance receipt";
pub const RECEIPT_FILE_TYPE_ERROR: &str = "acceptance receipt must be a regular file";
pub const RECEIPT_FILE_OWNER_ERROR: &str = "acceptance receipt owner mismatch";
pub const RECEIPT_FILE_MODE_ERROR: &str = "acceptance receipt mode mismatch";
pub const RECEIPT_FILE_LINK_ERROR: &str = "acceptance receipt must have one link";
pub const RECEIPT_FILE_ACL_ERROR: &str = "acceptance receipt ACL is not trusted";
pub const RECEIPT_PARSE_ERROR: &str = "parse signed acceptance receipt";
pub const RECEIPT_CANONICAL_ERROR: &str = "acceptance receipt is not canonical JSON";
pub const RECEIPT_DUPLICATE_FIELD_ERROR: &str = "acceptance receipt contains a duplicate field";
pub const RECEIPT_SIGNATURE_ERROR: &str = "acceptance receipt signature is invalid";
pub const RECEIPT_AUTHORITY_ERROR: &str = "acceptance receipt authority is not trusted";
pub const RECEIPT_SCHEMA_ERROR: &str = "acceptance receipt schema is unsupported";
pub const RECEIPT_PRODUCT_ERROR: &str = "acceptance receipt product mismatch";
pub const RECEIPT_BINDING_ERROR: &str = "acceptance receipt candidate binding mismatch";
pub const RECEIPT_CHECKLIST_ERROR: &str = "acceptance receipt checklist schema mismatch";
pub const RECEIPT_PROVIDER_ERROR: &str = "acceptance receipt provider binding mismatch";
pub const RECEIPT_OPERATOR_ERROR: &str = "acceptance receipt operator mismatch";
pub const RECEIPT_TIME_ERROR: &str = "acceptance receipt completion time is invalid";
pub const RECEIPT_EXPIRED_ERROR: &str = "acceptance receipt has expired";
pub const RECEIPT_FIELD_ERROR: &str = "acceptance receipt field is invalid";
pub const RECEIPT_DIGEST_ERROR: &str = "acceptance receipt digest is invalid";
pub const RECEIPT_TARGET_ERROR: &str = "acceptance receipt does not meet the Alpha target";

pub fn pending_manual_acceptance() -> ManualAcceptanceEvidence {
    ManualAcceptanceEvidence {
        schema: ACCEPTANCE_CHECKLIST_SCHEMA.to_owned(),
        status: PENDING_STATUS.to_owned(),
        checks: REQUIRED_ACCEPTANCE_CHECKS
            .iter()
            .map(|identifier| ManualAcceptanceCheck {
                identifier: (*identifier).to_owned(),
                status: PENDING_STATUS.to_owned(),
            })
            .collect(),
    }
}
use yolo_release_evidence_model::{ManualAcceptanceCheck, ManualAcceptanceEvidence};
