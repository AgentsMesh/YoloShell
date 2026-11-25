use super::super::*;
use super::test_support::{ProbeFixture, gone_identity};
use super::{guardian_id_from_name, read_exact_session};
use crate::paths::constants::MAX_PRIVATE_DIRECTORY_ENTRIES;
use crate::paths::test_support::retired_storage_artifact_name;
use crate::process::{ProcessIdentityRelation, install_test_process_identity_relation};
use std::ffi::{CString, OsStr};
use std::os::unix::ffi::OsStrExt;

const INVALID_ENTRY_NAME: &str = "not-a-guardian-probe";
const FIRST_STORAGE_ARTIFACT_INDEX: usize = 0;
const OVERFLOW_STORAGE_ARTIFACT_COUNT: usize = 1;
const STORAGE_ARTIFACT_TARGET_PREFIX: &str = "probe-budget-storage-artifact-";
const UNEXPECTED_OBSERVATION_DETAIL: &str = "unexpected observation failure";

#[test]
fn exact_live_probe_is_selected() {
    let fixture = ProbeFixture::new();

    assert_eq!(
        read_exact_session(
            &fixture.namespace,
            &fixture.paths,
            &fixture.record,
            fixture.identity,
        )
        .unwrap(),
        fixture.probe
    );
}

#[test]
fn proven_gone_stale_probe_is_ignored() {
    let fixture = ProbeFixture::new();
    let gone = gone_identity();
    fixture.write_probe(fixture.stale_probe(
        Uuid::new_v4(),
        gone,
        fixture.probe.clone().rebind_holder_for_test(gone),
    ));

    assert!(
        read_exact_session(
            &fixture.namespace,
            &fixture.paths,
            &fixture.record,
            fixture.identity,
        )
        .is_ok()
    );
}

#[test]
fn second_live_guardian_holding_the_pty_is_rejected() {
    let fixture = ProbeFixture::new();
    let holder = fixture.spawn_holder();
    let custody = fixture
        .probe
        .clone()
        .rebind_holder_for_test(holder.identity);
    assert!(
        custody
            .process_holds_pty_object(holder.identity.pid)
            .unwrap()
    );
    fixture.write_probe(fixture.stale_probe(Uuid::new_v4(), holder.identity, custody));

    assert!(
        read_exact_session(
            &fixture.namespace,
            &fixture.paths,
            &fixture.record,
            fixture.identity,
        )
        .is_err()
    );
}

#[test]
fn indeterminate_stale_guardian_is_rejected() {
    let fixture = ProbeFixture::new();
    let gone = gone_identity();
    fixture.write_probe(fixture.stale_probe(
        Uuid::new_v4(),
        gone,
        fixture.probe.clone().rebind_holder_for_test(gone),
    ));
    let _override = install_test_process_identity_relation(
        gone.pid,
        gone.start_time,
        ProcessIdentityRelation::Unknown,
    );

    assert!(
        read_exact_session(
            &fixture.namespace,
            &fixture.paths,
            &fixture.record,
            fixture.identity,
        )
        .is_err()
    );
}

#[test]
fn unexpected_probe_entry_is_rejected() {
    let fixture = ProbeFixture::new();
    fixture
        .namespace
        .write_json(
            &probe_directory(&fixture.paths, fixture.record.runtime.terminal_id)
                .join(INVALID_ENTRY_NAME),
            &fixture.record,
        )
        .unwrap();

    assert!(
        read_exact_session(
            &fixture.namespace,
            &fixture.paths,
            &fixture.record,
            fixture.identity,
        )
        .is_err()
    );
}

#[test]
fn probe_entry_names_must_be_canonical_json_uuids() {
    let guardian_id = Uuid::new_v4();
    let canonical = format!("{guardian_id}{PROBE_FILE_SUFFIX}");
    let uppercase = canonical.to_uppercase();

    assert_eq!(
        guardian_id_from_name(OsStr::new(&canonical)).unwrap(),
        guardian_id
    );
    assert!(guardian_id_from_name(OsStr::new(&uppercase)).is_err());
    assert!(guardian_id_from_name(OsStr::new(INVALID_ENTRY_NAME)).is_err());
}

#[test]
fn storage_artifacts_still_consume_the_probe_entry_budget() {
    let fixture = ProbeFixture::new();
    let directory = probe_directory(&fixture.paths, fixture.record.runtime.terminal_id);
    let final_index = MAX_PRIVATE_DIRECTORY_ENTRIES + OVERFLOW_STORAGE_ARTIFACT_COUNT;
    for index in FIRST_STORAGE_ARTIFACT_INDEX..=final_index {
        let target = CString::new(format!("{STORAGE_ARTIFACT_TARGET_PREFIX}{index}")).unwrap();
        let artifact = retired_storage_artifact_name(&target).unwrap();
        std::fs::write(directory.join(OsStr::from_bytes(artifact.as_bytes())), []).unwrap();
    }

    let error = read_exact_session(
        &fixture.namespace,
        &fixture.paths,
        &fixture.record,
        fixture.identity,
    )
    .unwrap_err();
    assert!(
        format!("{error:#}").contains(super::ENTRY_LIMIT_DETAIL),
        "{UNEXPECTED_OBSERVATION_DETAIL}: {error:#}"
    );
}
