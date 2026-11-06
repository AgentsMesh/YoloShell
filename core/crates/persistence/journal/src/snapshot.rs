use super::snapshot_format::{decode_snapshot, encode_snapshot};
use super::*;

#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum SnapshotWriteStage {
    TemporarySynced,
    Installed,
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum SnapshotReadStage {
    AfterFirstRead,
}

pub fn write_snapshot(path: &Path, snapshot: &TerminalSnapshot) -> Result<(), JournalError> {
    write_snapshot_with(path, snapshot, |_| Ok(()))
}

pub fn write_snapshot_retained(
    entry: PrivateFileEntry,
    snapshot: &TerminalSnapshot,
) -> Result<(), JournalError> {
    write_snapshot_entry_with(entry, snapshot, |_| Ok(()))
}

fn write_snapshot_with<F>(
    path: &Path,
    snapshot: &TerminalSnapshot,
    hook: F,
) -> Result<(), JournalError>
where
    F: FnMut(SnapshotWriteStageValue) -> Result<(), JournalError>,
{
    let entry = PrivateFileEntry::ensure(path)?;
    write_snapshot_entry_with(entry, snapshot, hook)
}

fn write_snapshot_entry_with<F>(
    entry: PrivateFileEntry,
    snapshot: &TerminalSnapshot,
    mut hook: F,
) -> Result<(), JournalError>
where
    F: FnMut(SnapshotWriteStageValue) -> Result<(), JournalError>,
{
    let path = entry.path();
    let encoded = encode_snapshot(snapshot)?;
    let _directory_lock = entry.lock_and_cleanup(SNAPSHOT_TEMPORARY_PREFIX, TEMPORARY_SUFFIX)?;
    let expected = entry.inspect(MAX_SNAPSHOT_DISK_BYTES)?;
    let mut temporary = entry.temporary(SNAPSHOT_TEMPORARY_PREFIX, TEMPORARY_SUFFIX)?;
    let result: Result<(), JournalError> = (|| {
        temporary.file_mut().write_all(&encoded)?;
        temporary.sync_and_inspect(path, MAX_SNAPSHOT_DISK_BYTES)?;
        hook(SnapshotWriteStageValue::TemporarySynced)?;
        entry.verify_expected(expected, MAX_SNAPSHOT_DISK_BYTES)?;
        entry.rename_temporary(&temporary, MAX_SNAPSHOT_DISK_BYTES)?;
        hook(SnapshotWriteStageValue::Installed)?;
        entry.verify_current_content_version(temporary.identity(), MAX_SNAPSHOT_DISK_BYTES)?;
        let installed = read_stable_bytes(&entry, |_| Ok(()))?;
        verify_installed_content(&installed, &encoded)?;
        Ok(())
    })();
    if let Err(error) = result {
        entry.remove_temporary(&temporary)?;
        return Err(error);
    }
    Ok(())
}

pub fn read_snapshot(path: &Path) -> Result<TerminalSnapshot, JournalError> {
    read_snapshot_with(path, |_| Ok(()))
}

fn read_snapshot_with<F>(path: &Path, hook: F) -> Result<TerminalSnapshot, JournalError>
where
    F: FnMut(SnapshotReadStageValue) -> Result<(), JournalError>,
{
    let entry = PrivateFileEntry::open(path)?;
    read_snapshot_entry_with(entry, hook)
}

fn read_snapshot_entry_with<F>(
    entry: PrivateFileEntry,
    hook: F,
) -> Result<TerminalSnapshot, JournalError>
where
    F: FnMut(SnapshotReadStageValue) -> Result<(), JournalError>,
{
    let bytes = read_stable_bytes(&entry, hook)?;
    decode_snapshot(&bytes)
}

fn read_stable_bytes<F>(entry: &PrivateFileEntry, mut hook: F) -> Result<Vec<u8>, JournalError>
where
    F: FnMut(SnapshotReadStageValue) -> Result<(), JournalError>,
{
    let mut opened = entry.open_read(MAX_SNAPSHOT_DISK_BYTES)?;
    let first = bounded_read(opened.file_mut())?;
    opened.identity().verify_file_bounded(
        opened.file(),
        u64::try_from(first.len()).map_err(|_| JournalError::PayloadTooLarge)?,
        entry.path(),
        MAX_SNAPSHOT_DISK_BYTES,
    )?;
    hook(SnapshotReadStageValue::AfterFirstRead)?;
    opened.file_mut().seek(SeekFrom::Start(0))?;
    let second = bounded_read(opened.file_mut())?;
    if first != second {
        return Err(anyhow::anyhow!("snapshot changed while reading").into());
    }
    opened.identity().verify_file_bounded(
        opened.file(),
        u64::try_from(second.len()).map_err(|_| JournalError::PayloadTooLarge)?,
        entry.path(),
        MAX_SNAPSHOT_DISK_BYTES,
    )?;
    entry.verify_current_unchanged(opened.identity(), MAX_SNAPSHOT_DISK_BYTES)?;
    Ok(first)
}

fn bounded_read(file: &mut File) -> Result<Vec<u8>, JournalError> {
    let limit = MAX_SNAPSHOT_DISK_BYTES
        .checked_add(INITIAL_RETAINED_SEQUENCE)
        .ok_or(JournalError::PayloadTooLarge)?;
    let mut bytes = Vec::new();
    file.take(limit).read_to_end(&mut bytes)?;
    if u64::try_from(bytes.len()).map_err(|_| JournalError::PayloadTooLarge)?
        > MAX_SNAPSHOT_DISK_BYTES
    {
        return Err(JournalError::PayloadTooLarge);
    }
    Ok(bytes)
}

fn verify_installed_content(installed: &[u8], expected: &[u8]) -> Result<(), JournalError> {
    if installed != expected {
        return Err(anyhow::anyhow!("installed snapshot content changed").into());
    }
    Ok(())
}

#[derive(Clone, Copy)]
enum SnapshotWriteStageValue {
    TemporarySynced,
    Installed,
}

#[derive(Clone, Copy)]
enum SnapshotReadStageValue {
    AfterFirstRead,
}

#[cfg(test)]
pub(super) fn write_snapshot_with_test_hook<F>(
    path: &Path,
    snapshot: &TerminalSnapshot,
    mut hook: F,
) -> Result<(), JournalError>
where
    F: FnMut(SnapshotWriteStage) -> Result<(), JournalError>,
{
    write_snapshot_with(path, snapshot, |stage| hook(stage.into()))
}

#[cfg(test)]
pub(super) fn read_snapshot_with_test_hook<F>(
    path: &Path,
    mut hook: F,
) -> Result<TerminalSnapshot, JournalError>
where
    F: FnMut(SnapshotReadStage) -> Result<(), JournalError>,
{
    read_snapshot_with(path, |stage| hook(stage.into()))
}

#[cfg(test)]
impl From<SnapshotWriteStageValue> for SnapshotWriteStage {
    fn from(value: SnapshotWriteStageValue) -> Self {
        match value {
            SnapshotWriteStageValue::TemporarySynced => Self::TemporarySynced,
            SnapshotWriteStageValue::Installed => Self::Installed,
        }
    }
}

#[cfg(test)]
impl From<SnapshotReadStageValue> for SnapshotReadStage {
    fn from(_: SnapshotReadStageValue) -> Self {
        Self::AfterFirstRead
    }
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/tests_snapshot_wave5.rs"]
mod wave5_tests;
