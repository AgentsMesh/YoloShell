use super::acceptance_receipt_support::{record, try_signed};
use crate::AcceptanceRecord;
use crate::constants::*;

const INVALID_DIGEST_CHARACTER: &str = "g";
const UPPERCASE_DIGEST_CHARACTER: &str = "A";
const UNSUPPORTED_PROVIDER: &str = "unsupported-provider";
const UNSUPPORTED_ARCHITECTURE: &str = "unsupported-architecture";
const OVERFLOW_MACOS_VERSION: &str = "65536.2";
const EMPTY_MACOS_COMPONENT: &str = "14..2";

#[test]
fn receipt_signing_rejects_malformed_record_fields_before_signing() {
    let mutations: &[fn(&mut AcceptanceRecord)] = &[
        |value| value.schema = ACCEPTANCE_RECORD_SCHEMA + 1,
        |value| value.product.clear(),
        |value| value.artifact_sha256 = INVALID_DIGEST_CHARACTER.repeat(SHA256_HEX_CHARACTERS),
        |value| value.provenance_sha256 = UPPERCASE_DIGEST_CHARACTER.repeat(SHA256_HEX_CHARACTERS),
        |value| value.operator_identity = "x".repeat(MAX_ACCEPTANCE_FIELD_BYTES + 1),
        |value| value.operator_identity.push('\n'),
        |value| value.desktop_os.name = "x".repeat(ACCEPTANCE_PLATFORM_NAME_MAX_BYTES + 1),
        |value| value.remote_os.architecture = UNSUPPORTED_ARCHITECTURE.to_owned(),
        |value| value.providers[0].provider = UNSUPPORTED_PROVIDER.to_owned(),
        |value| {
            value.providers = vec![value.providers[0].clone(); MAX_ACCEPTANCE_PROVIDER_ENTRIES + 1]
        },
        |value| {
            value.providers.pop();
        },
        |value| value.results.clear(),
        |value| value.results.reverse(),
        |value| value.results[0].identifier.clear(),
        |value| value.desktop_os.version = OVERFLOW_MACOS_VERSION.to_owned(),
        |value| value.desktop_os.version = EMPTY_MACOS_COMPONENT.to_owned(),
        |value| value.expires_at_unix_seconds = value.completed_at_unix_seconds,
        |value| {
            value.expires_at_unix_seconds =
                value.completed_at_unix_seconds + ACCEPTANCE_MAX_VALIDITY_SECONDS + 1
        },
    ];
    for mutate in mutations {
        let mut value = record();
        mutate(&mut value);
        assert!(try_signed(value).is_err());
    }
}
