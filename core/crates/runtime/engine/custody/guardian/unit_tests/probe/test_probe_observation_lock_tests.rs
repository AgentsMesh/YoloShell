use super::super::*;
use super::test_support::ProbeFixture;
use super::{
    ENTRY_VERSION_CHANGED, ObservationBoundary, read_exact_observation, read_exact_session,
    read_exact_session_observing,
};
use crate::paths::constants::MAX_DURABLE_JSON_BYTES;
use crate::paths::private_file::PrivateFileEntry;
use std::io::Write;

const UNEXPECTED_OBSERVATION_DETAIL: &str = "unexpected observation failure";

#[test]
fn exact_observation_retains_shared_probe_lock_until_drop() {
    let fixture = ProbeFixture::new();
    let observation = read_exact_observation(
        &fixture.namespace,
        &fixture.paths,
        &fixture.record,
        fixture.identity,
    )
    .unwrap();
    let target = current_probe_path(&fixture);
    assert!(exclusive_probe_lock(&target).is_err());
    drop(observation);
    assert!(exclusive_probe_lock(&target).is_ok());
}

#[test]
fn exact_observation_rejects_exclusive_probe_lock_contention() {
    let fixture = ProbeFixture::new();
    let (_opened, _exclusive) = exclusive_probe_lock(&current_probe_path(&fixture)).unwrap();

    let error = read_exact_session(
        &fixture.namespace,
        &fixture.paths,
        &fixture.record,
        fixture.identity,
    )
    .unwrap_err();
    assert!(
        format!("{error:#}").contains(ENTRY_VERSION_CHANGED),
        "{UNEXPECTED_OBSERVATION_DETAIL}: {error:#}"
    );
}

#[test]
fn exact_observation_rejects_same_inode_rewrite_with_restored_mtime() {
    let fixture = ProbeFixture::new();
    let target = current_probe_path(&fixture);
    let bytes = std::fs::read(&target).unwrap();
    let modified = target.metadata().unwrap().modified().unwrap();

    let result = read_exact_session_observing(
        &fixture.namespace,
        &fixture.paths,
        &fixture.record,
        fixture.identity,
        |boundary| {
            if boundary == ObservationBoundary::AfterEntryRead {
                let mut file = std::fs::OpenOptions::new().write(true).open(&target)?;
                file.write_all(&bytes)?;
                file.sync_all()?;
                file.set_times(std::fs::FileTimes::new().set_modified(modified))?;
            }
            Ok(())
        },
    );

    assert_error_contains(result.unwrap_err(), ENTRY_VERSION_CHANGED);
}

fn assert_error_contains(error: anyhow::Error, detail: &str) {
    assert!(
        format!("{error:#}").contains(detail),
        "{UNEXPECTED_OBSERVATION_DETAIL}: {error:#}"
    );
}

fn current_probe_path(fixture: &ProbeFixture) -> std::path::PathBuf {
    let guardian = fixture.record.armed_lease().unwrap().guardian.unwrap();
    path(
        &fixture.paths,
        fixture.record.runtime.terminal_id,
        guardian.guardian_id(),
    )
}

fn exclusive_probe_lock(
    target: &std::path::Path,
) -> anyhow::Result<(
    crate::paths::private_file::OpenedPrivateFile,
    crate::paths::process_file_lock::ProcessFileLock,
)> {
    use crate::paths::process_file_lock::{Mode, ProcessFileLock};
    let opened = PrivateFileEntry::open(target)?.open_retained(MAX_DURABLE_JSON_BYTES)?;
    let lock = ProcessFileLock::try_lock(
        opened.file(),
        opened.identity().object_identity(),
        Mode::Exclusive,
    )?
    .ok_or_else(|| anyhow::anyhow!(ENTRY_VERSION_CHANGED))?;
    Ok((opened, lock))
}
