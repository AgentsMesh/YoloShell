use super::tests_retained_hooks::{inject_parent_sync_failure, set_staging_hook};
use super::tests_retained_support::{RETAINED_PAYLOAD, RetainedFixture, saved_path};
use super::tests_security_support::{REPLACEMENT_BYTES, replace_target, temporary_path};
use super::*;
use std::fs::OpenOptions;
use std::io::{Seek, SeekFrom, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

const LAST_BYTE_OFFSET_FROM_END: i64 = -1;
const CORRUPTED_TAIL_BYTE: u8 = u8::MAX;
const PARTIAL_TAIL_TRUNCATION_BYTES: u64 = 3;
const EXTRA_TAIL_BYTES: &[u8] = b"unexpected tail";

#[test]
fn every_append_phase_recovers_once_after_namespace_loss() {
    for stage in [
        AppendStage::BeforeWrite,
        AppendStage::AfterRecordSync,
        AppendStage::AfterCursorCommit,
    ] {
        let mut fixture = RetainedFixture::fail_at(stage);
        let recovered = fixture.reconcile().unwrap().unwrap();
        assert_eq!(recovered.sequence, INITIAL_RETAINED_SEQUENCE);
        assert_eq!(recovered.data, RETAINED_PAYLOAD);
        fixture.assert_exactly_once();
        fixture.assert_replacements_unchanged();
    }
}

#[test]
fn mismatched_synced_tail_is_truncated_and_rewritten() {
    let mut fixture = RetainedFixture::fail_at(AppendStage::AfterRecordSync);
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&fixture.moved_journal)
        .unwrap();
    file.seek(SeekFrom::End(LAST_BYTE_OFFSET_FROM_END)).unwrap();
    file.write_all(&[CORRUPTED_TAIL_BYTE]).unwrap();
    file.sync_all().unwrap();

    fixture.reconcile().unwrap().unwrap();

    fixture.assert_exactly_once();
    fixture.assert_replacements_unchanged();
}

#[test]
fn partial_synced_tail_is_truncated_and_rewritten() {
    let mut fixture = RetainedFixture::fail_at(AppendStage::AfterRecordSync);
    let file = OpenOptions::new()
        .write(true)
        .open(&fixture.moved_journal)
        .unwrap();
    let length = file.metadata().unwrap().len();
    file.set_len(length - PARTIAL_TAIL_TRUNCATION_BYTES)
        .unwrap();
    file.sync_all().unwrap();

    fixture.reconcile().unwrap().unwrap();

    fixture.assert_exactly_once();
    fixture.assert_replacements_unchanged();
}

#[test]
fn extra_synced_tail_is_truncated_and_rewritten() {
    let mut fixture = RetainedFixture::fail_at(AppendStage::AfterRecordSync);
    let mut file = OpenOptions::new()
        .append(true)
        .open(&fixture.moved_journal)
        .unwrap();
    file.write_all(EXTRA_TAIL_BYTES).unwrap();
    file.sync_all().unwrap();

    fixture.reconcile().unwrap().unwrap();

    fixture.assert_exactly_once();
    fixture.assert_replacements_unchanged();
}

#[test]
fn cursor_parent_sync_failure_reloads_the_published_cursor() {
    let mut fixture = RetainedFixture::fail_at(AppendStage::AfterRecordSync);
    inject_parent_sync_failure();

    assert!(fixture.reconcile().is_err());
    let recovered = fixture.reconcile().unwrap().unwrap();

    assert_eq!(recovered.sequence, INITIAL_RETAINED_SEQUENCE);
    fixture.assert_exactly_once();
    fixture.assert_artifacts_bounded();
    fixture.assert_replacements_unchanged();
}

#[test]
fn replaced_cursor_staging_file_is_preserved_without_installing_it() {
    let mut fixture = RetainedFixture::fail_at(AppendStage::AfterRecordSync);
    let parent = fixture.moved_journal.parent().unwrap().to_path_buf();
    let observed = Arc::new(Mutex::new(None));
    let captured = Arc::clone(&observed);
    set_staging_hook(move || {
        let temporary = temporary_path(
            &parent,
            crate::paths::test_support::retired_artifact_prefix(),
        );
        replace_target(&temporary, &saved_path(&temporary));
        *captured.lock().unwrap() = Some(temporary);
        Ok(())
    });

    assert!(fixture.reconcile().is_err());

    let replacement: PathBuf = observed.lock().unwrap().take().unwrap();
    assert_eq!(std::fs::read(replacement).unwrap(), REPLACEMENT_BYTES);
    fixture.assert_replacements_unchanged();
}

#[test]
fn cursor_replacement_before_isolation_is_fenced_without_a_swap() {
    let mut fixture = RetainedFixture::fail_at(AppendStage::AfterRecordSync);
    let cursor = fixture.moved_cursor.clone();
    let saved = saved_path(&cursor);
    crate::paths::test_support::set_retained_exchange_hook(move || {
        replace_target(&cursor, &saved);
        Ok(())
    });

    assert!(fixture.reconcile().is_err());

    assert_eq!(
        std::fs::read(&fixture.moved_cursor).unwrap(),
        REPLACEMENT_BYTES
    );
    fixture.assert_artifacts_bounded();
    fixture.assert_replacements_unchanged();
}
