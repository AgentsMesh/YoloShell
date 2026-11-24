use super::super::*;
use super::test_support::{ProbeFixture, gone_identity};
use super::{
    DIRECTORY_REVISION_CHANGED, ENTRY_VERSION_CHANGED, ObservationBoundary, read_exact_observation,
    read_exact_session, read_exact_session_observing,
};
use std::path::PathBuf;

const DISPLACED_DIRECTORY_NAME: &str = "guardian-probes-displaced";
const REPLACEMENT_DIRECTORY_NAME: &str = "guardian-probes-replacement";
const BACKUP_FILE_SUFFIX: &str = ".backup";
const RETIRED_FILE_SUFFIX: &str = ".retired";
const UNEXPECTED_OBSERVATION_DETAIL: &str = "unexpected observation failure";

#[test]
fn directory_swap_back_is_rejected() {
    let fixture = ProbeFixture::new();
    let directory = probe_directory(&fixture.paths, fixture.record.runtime.terminal_id);
    let displaced = directory.with_file_name(DISPLACED_DIRECTORY_NAME);
    let replacement = directory.with_file_name(REPLACEMENT_DIRECTORY_NAME);
    let target = current_probe_path(&fixture);
    let malicious = exact_probe(
        &fixture,
        fixture.probe.clone().corrupt_descriptor_identity_for_test(),
    );

    let result = read_exact_session_observing(
        &fixture.namespace,
        &fixture.paths,
        &fixture.record,
        fixture.identity,
        |boundary| {
            if boundary == ObservationBoundary::AfterFirstScan {
                std::fs::rename(&directory, &displaced)?;
                fixture.namespace.ensure_directory(&directory)?;
                fixture.namespace.write_json(&target, &malicious)?;
            } else if boundary == ObservationBoundary::AfterEntryOpen {
                std::fs::rename(&directory, &replacement)?;
                std::fs::rename(&displaced, &directory)?;
            }
            Ok(())
        },
    );

    let error = result.unwrap_err();
    assert!(
        format!("{error:#}").contains(DIRECTORY_REVISION_CHANGED),
        "{UNEXPECTED_OBSERVATION_DETAIL}: {error:#}"
    );
}

#[test]
fn same_value_inode_replacement_restored_after_read_is_rejected() {
    let fixture = ProbeFixture::new();
    let target = current_probe_path(&fixture);
    let backup = path_with_suffix(&target, BACKUP_FILE_SUFFIX);
    let retired = path_with_suffix(&target, RETIRED_FILE_SUFFIX);
    let same_value = exact_probe(&fixture, fixture.probe.clone());

    let result = read_exact_session_observing(
        &fixture.namespace,
        &fixture.paths,
        &fixture.record,
        fixture.identity,
        |boundary| {
            if boundary == ObservationBoundary::AfterFirstScan {
                std::fs::rename(&target, &backup)?;
                fixture.namespace.write_json(&target, &same_value)?;
            } else if boundary == ObservationBoundary::AfterEntryRead {
                std::fs::rename(&target, &retired)?;
                std::fs::rename(&backup, &target)?;
                std::fs::remove_file(&retired)?;
            }
            Ok(())
        },
    );

    let error = result.unwrap_err();
    assert!(
        format!("{error:#}").contains(ENTRY_VERSION_CHANGED),
        "{UNEXPECTED_OBSERVATION_DETAIL}: {error:#}"
    );
}

#[test]
fn same_value_replacement_after_snapshot_changes_the_exact_observation() {
    let fixture = ProbeFixture::new();
    let target = current_probe_path(&fixture);
    let replacement = path_with_suffix(&target, RETIRED_FILE_SUFFIX);
    let before = read_exact_observation(
        &fixture.namespace,
        &fixture.paths,
        &fixture.record,
        fixture.identity,
    )
    .unwrap();
    let same_value = exact_probe(&fixture, fixture.probe.clone());
    fixture
        .namespace
        .write_json(&replacement, &same_value)
        .unwrap();
    std::fs::rename(&replacement, &target).unwrap();
    let after = read_exact_observation(
        &fixture.namespace,
        &fixture.paths,
        &fixture.record,
        fixture.identity,
    )
    .unwrap();

    assert_ne!(before, after);
}

#[test]
fn inserted_then_removed_entry_invalidates_the_snapshot() {
    let fixture = ProbeFixture::new();
    let gone = gone_identity();
    let guardian_id = Uuid::new_v4();
    let extra = fixture.stale_probe(
        guardian_id,
        gone,
        fixture.probe.clone().rebind_holder_for_test(gone),
    );
    let extra_path = path(
        &fixture.paths,
        fixture.record.runtime.terminal_id,
        guardian_id,
    );

    let result = read_exact_session_observing(
        &fixture.namespace,
        &fixture.paths,
        &fixture.record,
        fixture.identity,
        |boundary| {
            if boundary == ObservationBoundary::AfterFirstScan {
                fixture.namespace.write_json(&extra_path, &extra)?;
            } else if boundary == ObservationBoundary::BeforeSecondScan {
                std::fs::remove_file(&extra_path)?;
            }
            Ok(())
        },
    );

    let error = result.unwrap_err();
    assert!(
        format!("{error:#}").contains(DIRECTORY_REVISION_CHANGED),
        "{UNEXPECTED_OBSERVATION_DETAIL}: {error:#}"
    );
}

#[test]
fn tampered_descriptor_identity_is_rejected() {
    let fixture = ProbeFixture::new();
    let probe = exact_probe(
        &fixture,
        fixture.probe.clone().corrupt_descriptor_identity_for_test(),
    );
    std::fs::write(
        current_probe_path(&fixture),
        serde_json::to_vec_pretty(&probe).unwrap(),
    )
    .unwrap();

    let error = read_exact_session(
        &fixture.namespace,
        &fixture.paths,
        &fixture.record,
        fixture.identity,
    )
    .unwrap_err();
    assert!(
        format!("{error:#}").contains(EXACT_DESCRIPTOR_REQUIRED),
        "{UNEXPECTED_OBSERVATION_DETAIL}: {error:#}"
    );
}

fn exact_probe(fixture: &ProbeFixture, custody: PtyMasterFdCustodyProbe) -> GuardianCustodyProbe {
    GuardianCustodyProbe {
        binding: ProbeBinding::capture(&fixture.record, fixture.identity).unwrap(),
        custody,
    }
}

fn current_probe_path(fixture: &ProbeFixture) -> PathBuf {
    let guardian = fixture.record.armed_lease().unwrap().guardian.unwrap();
    path(
        &fixture.paths,
        fixture.record.runtime.terminal_id,
        guardian.guardian_id(),
    )
}

fn path_with_suffix(path: &std::path::Path, suffix: &str) -> PathBuf {
    let mut name = path.file_name().unwrap().to_os_string();
    name.push(suffix);
    path.with_file_name(name)
}
