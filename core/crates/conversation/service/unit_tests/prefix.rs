use crate::{evidence, prefix};
use std::{fs::OpenOptions, io::Write, os::unix::fs::FileExt};
use yolo_conversation_protocol::ConversationError;
use yolo_conversation_transcript::MAX_HISTORY_SCAN_BYTES;

const PREFIX: &[u8] = b"stable existing transcript\n";
const APPEND: &[u8] = b"new transcript record\n";

#[test]
fn append_during_read_preserves_the_bounded_prefix_and_does_not_cache_future_bytes() {
    let file = tempfile::NamedTempFile::new().unwrap();
    file.as_file().write_all(PREFIX).unwrap();
    let length = PREFIX.len() as u64;
    let initial_stamp = evidence::snapshot_stamp(file.as_file(), length)
        .unwrap()
        .unwrap();
    let value = prefix::guard(file.as_file(), length, |source, length| {
        let digest = prefix::hash(source, length)?;
        OpenOptions::new()
            .append(true)
            .open(file.path())?
            .write_all(APPEND)?;
        Ok((digest.clone(), Some(digest)))
    })
    .unwrap();
    assert_eq!(value, yolo_crypto::sha256_hex(PREFIX));
    assert_ne!(initial_stamp, evidence::stamp(file.as_file()).unwrap());
    assert_eq!(
        evidence::snapshot_stamp(file.as_file(), length).unwrap(),
        None
    );
    assert_eq!(prefix::hash(file.as_file(), length).unwrap(), value);
}

#[test]
fn rewrite_truncate_and_scan_limits_are_explicit_failures() {
    for truncate in [false, true] {
        let file = tempfile::NamedTempFile::new().unwrap();
        file.as_file().write_all(PREFIX).unwrap();
        let result = prefix::guard(file.as_file(), PREFIX.len() as u64, |source, length| {
            let digest = prefix::hash(source, length)?;
            if truncate {
                source.set_len(0)?;
            } else {
                source.write_all_at(b"changed", 0)?;
            }
            Ok(((), Some(digest)))
        });
        assert_eq!(
            result.unwrap_err().downcast_ref::<ConversationError>(),
            Some(&ConversationError::EvidenceChanged)
        );
    }
    let file = tempfile::NamedTempFile::new().unwrap();
    assert_eq!(
        prefix::hash(file.as_file(), MAX_HISTORY_SCAN_BYTES + 1)
            .unwrap_err()
            .downcast_ref::<ConversationError>(),
        Some(&ConversationError::HistoryLimitExceeded)
    );
}

#[test]
fn a_continuously_appending_writer_does_not_require_idle_before_a_prefix_can_publish() {
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };
    let file = tempfile::NamedTempFile::new().unwrap();
    file.as_file().write_all(PREFIX).unwrap();
    let stop = Arc::new(AtomicBool::new(false));
    let (started, receiver) = std::sync::mpsc::sync_channel(1);
    let writer = OpenOptions::new().append(true).open(file.path()).unwrap();
    let active = stop.clone();
    let thread = std::thread::spawn(move || {
        let mut writer = writer;
        writer.write_all(APPEND).unwrap();
        started.send(()).unwrap();
        while !active.load(Ordering::Acquire) {
            writer.write_all(APPEND).unwrap();
            std::thread::yield_now();
        }
    });
    receiver.recv().unwrap();
    let outcome = prefix::guard(file.as_file(), PREFIX.len() as u64, |source, length| {
        let digest = prefix::hash(source, length)?;
        source.write_all_at(APPEND, source.metadata()?.len())?;
        Ok((digest.clone(), Some(digest)))
    });
    stop.store(true, Ordering::Release);
    thread.join().unwrap();
    assert_eq!(outcome.unwrap(), yolo_crypto::sha256_hex(PREFIX));
}
