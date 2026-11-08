use crate::digest::{
    MAX_DIGEST_INPUT_BYTES, bytes_sha256, copy_opened_bounded, opened_file_sha256,
    opened_file_sha256_after_capture,
};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::os::unix::fs::PermissionsExt;

const CONTENT: &[u8] = b"Yolo release evidence\n";
const APPENDED_CONTENT: &[u8] = b"changed";
const CONTENT_SHA256: &str = "090c3dfc3aad04f3c83666e0ab21bcd2b3ae8a8ae01d85a078acb87cf00b391c";
const COPY_LIMIT_BYTES: u64 = 1024;
const CAPTURE_FAILURE: &str = "injected artifact capture failure";
const FIXED_MODIFIED_SECONDS: u64 = 1_000;
const INITIAL_NANOSECONDS: u32 = 1;
const CHANGED_NANOSECONDS: u32 = 2;
const OWNER_READ_ONLY: u32 = 0o400;
const OWNER_READ_WRITE: u32 = 0o600;

#[test]
fn hashes_bytes_and_opened_files_identically() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("evidence");
    fs::write(&path, CONTENT).unwrap();
    assert_eq!(bytes_sha256(CONTENT), CONTENT_SHA256);
    assert_eq!(
        opened_file_sha256(&File::open(path).unwrap()).unwrap(),
        CONTENT_SHA256
    );
}

#[test]
fn bounded_copy_uses_the_retained_source_descriptor() {
    let root = tempfile::tempdir().unwrap();
    let source_path = root.path().join("source");
    let destination_path = root.path().join("destination");
    fs::write(&source_path, CONTENT).unwrap();
    let source = File::open(source_path).unwrap();
    let mut destination = File::create(&destination_path).unwrap();
    copy_opened_bounded(&source, &mut destination, COPY_LIMIT_BYTES, "fixture").unwrap();
    drop(destination);
    let mut copied = Vec::new();
    File::open(destination_path)
        .unwrap()
        .read_to_end(&mut copied)
        .unwrap();
    assert_eq!(copied, CONTENT);
}

#[test]
fn rejects_a_file_that_changes_after_metadata_capture() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("evidence");
    fs::write(&path, CONTENT).unwrap();
    let reader = File::open(&path).unwrap();
    let mut writer = OpenOptions::new().append(true).open(path).unwrap();
    let error = opened_file_sha256_after_capture(&reader, || {
        writer.write_all(APPENDED_CONTENT)?;
        writer.sync_all()?;
        Ok(())
    })
    .unwrap_err();
    assert!(error.to_string().contains("changed while it was read"));
}

#[test]
fn rejects_non_files_and_oversized_inputs_before_reading() {
    let root = tempfile::tempdir().unwrap();
    assert!(opened_file_sha256(&File::open(root.path()).unwrap()).is_err());
    let path = root.path().join("oversized");
    let file = File::create(path).unwrap();
    file.set_len(MAX_DIGEST_INPUT_BYTES + 1).unwrap();
    assert!(opened_file_sha256(&file).is_err());
}

#[test]
fn digest_rejects_truncation_read_failure_and_capture_failure() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("evidence");
    fs::write(&path, CONTENT).unwrap();
    let reader = File::open(&path).unwrap();
    assert!(opened_file_sha256_after_capture(&reader, || anyhow::bail!(CAPTURE_FAILURE)).is_err());
    let writer = OpenOptions::new().write(true).open(&path).unwrap();
    assert!(opened_file_sha256(&writer).is_err());
    let error = opened_file_sha256_after_capture(&reader, || {
        writer.set_len(u64::default())?;
        Ok(())
    })
    .unwrap_err();
    assert!(error.to_string().contains("truncated while it was read"));
    assert_eq!(opened_file_sha256(&reader).unwrap(), bytes_sha256(&[]));
}

#[test]
fn same_content_digests_reject_changed_modification_times() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("evidence");
    fs::write(&path, CONTENT).unwrap();
    let reader = File::open(&path).unwrap();
    let writer = OpenOptions::new().write(true).open(path).unwrap();
    let initial = std::time::UNIX_EPOCH
        + std::time::Duration::new(FIXED_MODIFIED_SECONDS, INITIAL_NANOSECONDS);
    for next in [
        std::time::UNIX_EPOCH,
        std::time::UNIX_EPOCH
            + std::time::Duration::new(FIXED_MODIFIED_SECONDS, CHANGED_NANOSECONDS),
    ] {
        writer
            .set_times(std::fs::FileTimes::new().set_modified(initial))
            .unwrap();
        let error = opened_file_sha256_after_capture(&reader, || {
            writer.set_times(std::fs::FileTimes::new().set_modified(next))?;
            Ok(())
        })
        .unwrap_err();
        assert!(error.to_string().contains("changed while it was read"));
    }
}

#[test]
fn same_content_digest_rejects_metadata_only_changes() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("evidence");
    fs::write(&path, CONTENT).unwrap();
    let reader = File::open(&path).unwrap();
    reader
        .set_permissions(fs::Permissions::from_mode(OWNER_READ_WRITE))
        .unwrap();
    let error = opened_file_sha256_after_capture(&reader, || {
        reader.set_permissions(fs::Permissions::from_mode(OWNER_READ_ONLY))?;
        Ok(())
    })
    .unwrap_err();
    assert!(error.to_string().contains("changed while it was read"));
    assert_eq!(fs::read(path).unwrap(), CONTENT);
}

#[test]
fn bounded_copy_rejects_invalid_types_limits_positions_and_destination_modes() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("source");
    let destination = root.path().join("destination");
    fs::write(&path, CONTENT).unwrap();
    let mut source = File::open(&path).unwrap();
    let mut writer = File::create(&destination).unwrap();
    assert!(
        copy_opened_bounded(
            &File::open(root.path()).unwrap(),
            &mut writer,
            COPY_LIMIT_BYTES,
            "fixture"
        )
        .is_err()
    );
    assert!(copy_opened_bounded(&source, &mut writer, u64::default(), "fixture").is_err());
    let mut readonly = File::open(destination).unwrap();
    assert!(copy_opened_bounded(&source, &mut readonly, COPY_LIMIT_BYTES, "fixture").is_err());
    source.seek(SeekFrom::End(i64::default())).unwrap();
    let error = copy_opened_bounded(&source, &mut writer, COPY_LIMIT_BYTES, "fixture").unwrap_err();
    assert!(error.to_string().contains("truncated while copied"));
}
