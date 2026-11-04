use super::*;

#[test]
fn unknown_boot_identity_cannot_create_managed_artifact_authority() {
    assert!(require_known_boot("unknown:test").is_err());
    assert!(require_known_boot("known-test-boot").is_ok());
}

const BOOT_ID: &str = "test-boot";
const PID: u32 = 42;
const START_TIME: u64 = 84;
const UNSUPPORTED_TIME_BOOT_ID: &str = "macos:123:456";

#[test]
fn artifact_owner_name_round_trips_exactly() {
    let owner = Owner::from_values(BOOT_ID, PID, START_TIME, Uuid::new_v4());
    assert_eq!(Owner::parse(OsStr::new(&owner.file_name())), Some(owner));
}

#[test]
fn untyped_boot_hash_cannot_declare_a_live_artifact_owner_gone() {
    let process = crate::process::ProcessIdentity::capture(std::process::id()).unwrap();
    let invocation = Uuid::new_v4();
    let current = Owner::current(invocation).unwrap();
    assert_eq!(current.process_identity(), process);
    assert_eq!(
        current.process_relation(),
        crate::process::ProcessIdentityRelation::Matching
    );
    let raw_digest = yolo_crypto::sha256_hex(UNSUPPORTED_TIME_BOOT_ID.as_bytes());
    let old_name = format!(
        "{CLAUDE_SETTINGS_FILE_PREFIX}{KNOWN_BOOT}{FIELD_SEPARATOR}{raw_digest}{FIELD_SEPARATOR}{}{FIELD_SEPARATOR}{}{FIELD_SEPARATOR}{invocation}{CLAUDE_SETTINGS_FILE_SUFFIX}",
        process.pid, process.start_time,
    );
    assert!(Owner::parse(OsStr::new(&old_name)).is_none());
    for name in [
        old_name.clone(),
        format!("{old_name}{HANDSHAKE_FAILURE_SUFFIX}"),
    ] {
        assert!(Owner::parse_artifact(OsStr::new(&name)).is_none());
    }
    assert_eq!(
        Owner::parse_artifact(OsStr::new(&current.file_name())),
        Some(current)
    );
    assert!(process.matches());
}

#[test]
fn malformed_names_are_never_cleanup_candidates() {
    for name in [
        "settings.json",
        "settings_k_bad_42_84_not-a-uuid.json",
        "settings_x_0000000000000000000000000000000000000000000000000000000000000000_42_84_123e4567-e89b-42d3-a456-426614174000.json",
        "settings_k_ABCDEF0000000000000000000000000000000000000000000000000000000000_42_84_123e4567-e89b-42d3-a456-426614174000.json",
        "settings_k_aBcdef0000000000000000000000000000000000000000000000000000000000_42_84_123e4567-e89b-42d3-a456-426614174000.json",
    ] {
        assert!(Owner::parse(OsStr::new(name)).is_none(), "{name}");
    }
}

#[test]
fn noncanonical_owner_aliases_are_never_cleanup_candidates() {
    let invocation = Uuid::parse_str("123e4567-e89b-42d3-a456-426614174000").unwrap();
    let owner = Owner::from_values(BOOT_ID, PID, START_TIME, invocation);
    let canonical = owner.file_name();
    let coordinates = format!("_{PID}_{START_TIME}_");
    let uppercase_uuid = invocation.hyphenated().to_string().to_uppercase();
    let compact_uuid = invocation.simple().to_string();
    for alias in [
        canonical.replacen(&coordinates, &format!("_0{PID}_{START_TIME}_"), 1),
        canonical.replacen(&coordinates, &format!("_{PID}_0{START_TIME}_"), 1),
        canonical.replacen(&invocation.to_string(), &uppercase_uuid, 1),
        canonical.replacen(&invocation.to_string(), &compact_uuid, 1),
        format!("{canonical}.extra"),
    ] {
        assert!(
            Owner::parse_artifact(OsStr::new(&alias)).is_none(),
            "{alias}"
        );
    }
}

#[test]
fn zero_process_coordinates_are_never_cleanup_candidates() {
    let owner = Owner::from_values(BOOT_ID, PID, START_TIME, Uuid::new_v4());
    let canonical = owner.file_name();
    let coordinates = format!("_{PID}_{START_TIME}_");
    for alias in [
        canonical.replacen(&coordinates, &format!("_0_{START_TIME}_"), 1),
        canonical.replacen(&coordinates, &format!("_{PID}_0_"), 1),
    ] {
        assert!(
            Owner::parse_artifact(OsStr::new(&alias)).is_none(),
            "{alias}"
        );
    }
}

#[test]
fn failure_owner_name_requires_an_exact_round_trip() {
    let owner = Owner::from_values(BOOT_ID, PID, START_TIME, Uuid::new_v4());
    let failure = owner.handshake_failure_file_name();
    assert_eq!(Owner::parse_artifact(OsStr::new(&failure)), Some(owner));
    assert!(
        Owner::parse_artifact(OsStr::new(&format!("{failure}{HANDSHAKE_FAILURE_SUFFIX}")))
            .is_none()
    );
}

#[test]
fn full_artifact_scan_uses_one_current_boot_identity() {
    let expected_boot = crate::process::boot_identity::BootIdentity::current();
    for ordinal in 0..MAX_ARTIFACTS {
        let owner = Owner::for_test(
            PID + u32::try_from(ordinal).unwrap(),
            START_TIME + u64::try_from(ordinal).unwrap(),
            Uuid::new_v4(),
        );
        assert_eq!(owner.boot, expected_boot);
    }
}
