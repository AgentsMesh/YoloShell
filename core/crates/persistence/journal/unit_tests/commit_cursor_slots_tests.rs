use super::*;
use std::fs::OpenOptions;
use std::io::{Seek, SeekFrom, Write};
use std::os::unix::fs::OpenOptionsExt;

const CURSOR_TEST_PATH: &str = "retained-cursor-slot-test";
const EARLIER_COMMITTED_BYTES: u64 = 128;
const LATER_COMMITTED_BYTES: u64 = 256;
const ROLLED_BACK_COMMITTED_BYTES: u64 = 96;
const EARLIER_REVISION: u64 = 4;
const LATER_REVISION: u64 = 5;
const NEXT_REVISION: u64 = LATER_REVISION + CURSOR_REVISION_INCREMENT;
const FIRST_CURSOR_SLOT_OFFSET: u64 = 0;
const FIRST_WRITE_INDEX: usize = 0;
const SECOND_WRITE_INDEX: usize = 1;
const SINGLE_SLOT_WRITE_COUNT: usize = 1;
const GENERATION_MIGRATION_WRITE_COUNT: usize = 2;

struct CursorTestFile {
    _root: tempfile::TempDir,
    file: File,
    identity: FileIdentity,
}

#[test]
fn active_slot_uses_revision_instead_of_committed_bytes() {
    let generation = Uuid::new_v4();
    let earlier = record(generation, LATER_COMMITTED_BYTES, EARLIER_REVISION);
    let rollback = record(generation, ROLLED_BACK_COMMITTED_BYTES, LATER_REVISION);
    let cursor = cursor_file(earlier, rollback);

    let active =
        read_active_cursor(&cursor.file, cursor.identity, Path::new(CURSOR_TEST_PATH)).unwrap();

    assert_eq!(active, rollback);
}

#[test]
fn corrupt_canonical_slot_fails_closed_while_staging_rewrites_it() {
    let generation = Uuid::new_v4();
    let current = record(generation, EARLIER_COMMITTED_BYTES, EARLIER_REVISION);
    let preserved = record(generation, LATER_COMMITTED_BYTES, LATER_REVISION);
    let mut cursor = cursor_file(current, preserved);
    cursor
        .file
        .seek(SeekFrom::Start(FIRST_CURSOR_SLOT_OFFSET))
        .unwrap();
    cursor.file.write_all(&[0_u8; COMMIT_CURSOR_BYTES]).unwrap();
    cursor.file.sync_all().unwrap();
    let identity = inspect(&cursor.file);

    assert!(
        read_active_cursor(&cursor.file, identity, Path::new(CURSOR_TEST_PATH)).is_err()
    );
    let plan = plan_retained_stage(
        &cursor.file,
        identity,
        Path::new(CURSOR_TEST_PATH),
        preserved,
        LATER_COMMITTED_BYTES,
    )
    .unwrap();

    assert_eq!(plan.writes.len(), SINGLE_SLOT_WRITE_COUNT);
    assert_eq!(
        plan.writes[FIRST_WRITE_INDEX].offset,
        FIRST_CURSOR_SLOT_OFFSET
    );
    assert_eq!(plan.record.publication_revision, NEXT_REVISION);
}

#[test]
fn corrupt_newer_slot_cannot_fall_back_to_the_older_committed_cursor() {
    let generation = Uuid::new_v4();
    let earlier = record(generation, EARLIER_COMMITTED_BYTES, EARLIER_REVISION);
    let later = record(generation, LATER_COMMITTED_BYTES, LATER_REVISION);
    let mut cursor = cursor_file(earlier, later);
    cursor
        .file
        .seek(SeekFrom::Start(COMMIT_CURSOR_BYTES as u64))
        .unwrap();
    cursor.file.write_all(&[0_u8; COMMIT_CURSOR_BYTES]).unwrap();
    cursor.file.sync_all().unwrap();
    let identity = inspect(&cursor.file);

    assert!(
        read_active_cursor(&cursor.file, identity, Path::new(CURSOR_TEST_PATH)).is_err()
    );
}

#[test]
fn conflicting_records_with_the_same_revision_fail_closed() {
    let generation = Uuid::new_v4();
    let first = record(generation, EARLIER_COMMITTED_BYTES, LATER_REVISION);
    let second = record(generation, LATER_COMMITTED_BYTES, LATER_REVISION);
    let cursor = cursor_file(first, second);

    assert!(
        read_active_cursor(&cursor.file, cursor.identity, Path::new(CURSOR_TEST_PATH)).is_err()
    );
}

#[test]
fn generation_migration_resumes_after_the_first_slot_is_synced() {
    let old_generation = Uuid::new_v4();
    let current_generation = Uuid::new_v4();
    let old_first = record(old_generation, EARLIER_COMMITTED_BYTES, EARLIER_REVISION);
    let old_second = record(old_generation, LATER_COMMITTED_BYTES, LATER_REVISION);
    let current = record(
        current_generation,
        EARLIER_COMMITTED_BYTES,
        INITIAL_CURSOR_REVISION,
    );
    let mut cursor = cursor_file(old_first, old_second);
    let first_plan = plan_retained_stage(
        &cursor.file,
        cursor.identity,
        Path::new(CURSOR_TEST_PATH),
        current,
        LATER_COMMITTED_BYTES,
    )
    .unwrap();
    assert_eq!(first_plan.writes.len(), GENERATION_MIGRATION_WRITE_COUNT);
    apply_write(&mut cursor.file, first_plan.writes[FIRST_WRITE_INDEX]);
    let interrupted_identity = inspect(&cursor.file);

    let resumed = plan_retained_stage(
        &cursor.file,
        interrupted_identity,
        Path::new(CURSOR_TEST_PATH),
        current,
        LATER_COMMITTED_BYTES,
    )
    .unwrap();

    assert_eq!(resumed.writes.len(), SINGLE_SLOT_WRITE_COUNT);
    assert_eq!(
        resumed.writes[FIRST_WRITE_INDEX].offset,
        first_plan.writes[SECOND_WRITE_INDEX].offset
    );
    apply_write(&mut cursor.file, resumed.writes[FIRST_WRITE_INDEX]);
    let installed = read_active_cursor(
        &cursor.file,
        inspect(&cursor.file),
        Path::new(CURSOR_TEST_PATH),
    )
    .unwrap();
    assert_eq!(installed, resumed.record);
}

fn record(generation: Uuid, committed_bytes: u64, publication_revision: u64) -> CursorRecord {
    CursorRecord {
        generation,
        committed_bytes,
        publication_revision,
    }
}

fn cursor_file(first: CursorRecord, second: CursorRecord) -> CursorTestFile {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join(CURSOR_TEST_PATH);
    let mut file = OpenOptions::new()
        .create_new(true)
        .read(true)
        .write(true)
        .mode(crate::paths::constants::PRIVATE_FILE_MODE)
        .open(path)
        .unwrap();
    file.write_all(&encode_cursor(first)).unwrap();
    file.write_all(&encode_cursor(second)).unwrap();
    file.sync_all().unwrap();
    let identity = inspect(&file);
    CursorTestFile {
        _root: root,
        file,
        identity,
    }
}

fn inspect(file: &File) -> FileIdentity {
    FileIdentity::inspect_file_bounded(
        file,
        Path::new(CURSOR_TEST_PATH),
        RETAINED_CURSOR_BYTES as u64,
    )
    .unwrap()
}

fn apply_write(file: &mut File, write: CursorSlotWrite) {
    file.seek(SeekFrom::Start(write.offset)).unwrap();
    file.write_all(&encode_cursor(write.record)).unwrap();
    file.sync_all().unwrap();
}

#[path = "commit_cursor_slots_branch_matrix.rs"]
mod branch_matrix;
