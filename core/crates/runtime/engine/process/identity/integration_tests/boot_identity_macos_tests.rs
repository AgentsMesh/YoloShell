use super::*;

const UNSUPPORTED_TIME_BOOT_ID: &str = "macos:123:456";
const CURRENT_SESSION_BOOT_ID: &str = "macos:aaaaaaaa-bbbb-4ccc-8ddd-eeeeeeeeeeee";
const FOREIGN_SESSION_BOOT_ID: &str = "macos:bbbbbbbb-cccc-4ddd-8eee-ffffffffffff";
const NIL_SESSION_UUID_TEXT: &str = "00000000-0000-0000-0000-000000000000";
const SESSION_UUID_TEXT: &str = "aaaaaaaa-bbbb-4ccc-8ddd-eeeeeeeeeeee";

#[test]
fn session_bytes_require_a_hyphenated_non_nil_uuid_and_terminator() {
    let mut valid = SESSION_UUID_TEXT.as_bytes().to_vec();
    valid.push(NUL_BYTE);
    let parsed = parse_session_bytes(&valid).unwrap();

    assert_eq!(parsed, uuid::Uuid::parse_str(SESSION_UUID_TEXT).unwrap());
    assert!(parse_session_bytes(SESSION_UUID_TEXT.as_bytes()).is_none());
    let mut nil = NIL_SESSION_UUID_TEXT.as_bytes().to_vec();
    nil.push(NUL_BYTE);
    assert!(parse_session_bytes(&nil).is_none());
    assert!(parse_session_bytes(&[NUL_BYTE]).is_none());
}

#[test]
fn boot_identity_requires_a_non_nil_session_uuid() {
    assert_eq!(
        parse(CURRENT_SESSION_BOOT_ID),
        uuid::Uuid::parse_str(SESSION_UUID_TEXT).ok()
    );
    assert!(parse(UNSUPPORTED_TIME_BOOT_ID).is_none());
    assert!(parse(&format!("{MACOS_BOOT_PREFIX}{NIL_SESSION_UUID_TEXT}")).is_none());
}

#[test]
fn only_exact_session_uuids_prove_the_same_boot() {
    assert_eq!(
        relation(CURRENT_SESSION_BOOT_ID, CURRENT_SESSION_BOOT_ID),
        BootIdentityRelation::Same
    );
    assert_eq!(
        relation(CURRENT_SESSION_BOOT_ID, FOREIGN_SESSION_BOOT_ID),
        BootIdentityRelation::Different
    );
    for (left, right) in [
        (UNSUPPORTED_TIME_BOOT_ID, CURRENT_SESSION_BOOT_ID),
        (CURRENT_SESSION_BOOT_ID, UNSUPPORTED_TIME_BOOT_ID),
        (UNSUPPORTED_TIME_BOOT_ID, UNSUPPORTED_TIME_BOOT_ID),
    ] {
        assert_eq!(relation(left, right), BootIdentityRelation::Unknown);
    }
}
