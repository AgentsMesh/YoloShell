use super::tests_retained_hooks::{
    inject_parent_sync_failure, set_parent_sync_hook, set_post_install_hook,
};
use super::tests_retained_support::{RetainedFixture, saved_path};
use super::tests_security_support::{REPLACEMENT_BYTES, replace_target, write_private};
use super::*;
use std::fs;

const CLEANUP_SYNC_FAULT: &str = "injected retained cleanup parent sync failure";
const POST_INSTALL_FAULT: &str = "injected retained post-install failure";

#[test]
fn cleanup_parent_sync_failure_retries_without_leaking_a_cursor_temp() {
    let mut fixture = RetainedFixture::fail_at(AppendStage::AfterRecordSync);
    crate::paths::test_support::set_retained_cleanup_sync_hook(|| {
        Err(std::io::Error::other(CLEANUP_SYNC_FAULT).into())
    });

    assert!(fixture.reconcile().is_err());
    fixture.reconcile().unwrap().unwrap();

    fixture.assert_exactly_once();
    fixture.assert_artifacts_bounded();
    fixture.assert_replacements_unchanged();
}

#[test]
fn target_created_after_cursor_isolation_is_never_modified() {
    let mut fixture = RetainedFixture::fail_at(AppendStage::AfterRecordSync);
    let previous_cursor = fs::read(&fixture.moved_cursor).unwrap();
    let cursor = fixture.moved_cursor.clone();
    let observed = cursor.clone();
    crate::paths::test_support::set_retained_install_hook(move || {
        write_private(&cursor, REPLACEMENT_BYTES);
        Ok(())
    });

    assert!(fixture.reconcile().is_err());

    assert_eq!(fs::read(observed).unwrap(), REPLACEMENT_BYTES);
    fixture.assert_retired_cursor(&previous_cursor);
    fixture.assert_artifacts_bounded();
    fixture.assert_replacements_unchanged();
}

#[test]
fn journal_replacement_before_cursor_install_restores_the_old_cursor() {
    let mut fixture = RetainedFixture::fail_at(AppendStage::AfterRecordSync);
    let journal = fixture.moved_journal.clone();
    let saved = saved_path(&journal);
    let saved_for_hook = saved.clone();
    crate::paths::test_support::set_retained_install_hook(move || {
        replace_target(&journal, &saved_for_hook);
        Ok(())
    });

    assert!(fixture.reconcile().is_err());
    assert_eq!(fs::read(&fixture.moved_journal).unwrap(), REPLACEMENT_BYTES);
    fs::remove_file(&fixture.moved_journal).unwrap();
    fs::rename(saved, &fixture.moved_journal).unwrap();
    fixture.reconcile().unwrap().unwrap();

    fixture.assert_exactly_once();
    fixture.assert_artifacts_bounded();
    fixture.assert_replacements_unchanged();
}

#[test]
fn journal_replacement_after_cursor_install_rolls_the_cursor_back() {
    let mut fixture = RetainedFixture::fail_at(AppendStage::AfterRecordSync);
    let previous_cursor = fs::read(&fixture.moved_cursor).unwrap();
    let journal = fixture.moved_journal.clone();
    let saved = saved_path(&journal);
    let saved_for_hook = saved.clone();
    set_post_install_hook(move || {
        replace_target(&journal, &saved_for_hook);
        Ok(())
    });

    assert!(fixture.reconcile().is_err());
    assert_eq!(fs::read(&fixture.moved_cursor).unwrap(), previous_cursor);
    fs::remove_file(&fixture.moved_journal).unwrap();
    fs::rename(saved, &fixture.moved_journal).unwrap();
    fixture.reconcile().unwrap().unwrap();

    fixture.assert_exactly_once();
    fixture.assert_artifacts_bounded();
    fixture.assert_replacements_unchanged();
}

#[test]
fn journal_replacement_during_parent_sync_rolls_the_cursor_back() {
    let mut fixture = RetainedFixture::fail_at(AppendStage::AfterRecordSync);
    let previous_cursor = fs::read(&fixture.moved_cursor).unwrap();
    let journal = fixture.moved_journal.clone();
    let saved = saved_path(&journal);
    let saved_for_hook = saved.clone();
    set_parent_sync_hook(move || {
        replace_target(&journal, &saved_for_hook);
        Ok(())
    });

    assert!(fixture.reconcile().is_err());
    assert_eq!(fs::read(&fixture.moved_cursor).unwrap(), previous_cursor);
    fs::remove_file(&fixture.moved_journal).unwrap();
    fs::rename(saved, &fixture.moved_journal).unwrap();
    fixture.reconcile().unwrap().unwrap();

    fixture.assert_exactly_once();
    fixture.assert_artifacts_bounded();
    fixture.assert_replacements_unchanged();
}

#[test]
fn rollback_sync_failure_keeps_the_restored_cursor_capability() {
    let mut fixture = RetainedFixture::fail_at(AppendStage::AfterRecordSync);
    let journal = fixture.moved_journal.clone();
    let saved = saved_path(&journal);
    let saved_for_hook = saved.clone();
    set_post_install_hook(move || {
        replace_target(&journal, &saved_for_hook);
        Ok(())
    });
    inject_parent_sync_failure();

    assert!(fixture.reconcile().is_err());
    fs::remove_file(&fixture.moved_journal).unwrap();
    fs::rename(saved, &fixture.moved_journal).unwrap();
    fixture.reconcile().unwrap().unwrap();

    fixture.assert_exactly_once();
    fixture.assert_artifacts_bounded();
    fixture.assert_replacements_unchanged();
}

#[test]
fn installed_cursor_is_adopted_before_post_install_validation() {
    let mut fixture = RetainedFixture::fail_at(AppendStage::AfterRecordSync);
    let cursor = fixture.moved_cursor.clone();
    let saved = saved_path(&cursor);
    let saved_for_hook = saved.clone();
    set_post_install_hook(move || {
        replace_target(&cursor, &saved_for_hook);
        Ok(())
    });

    assert!(fixture.reconcile().is_err());
    fs::remove_file(&fixture.moved_cursor).unwrap();
    fs::rename(saved, &fixture.moved_cursor).unwrap();
    fixture.reconcile().unwrap().unwrap();

    fixture.assert_exactly_once();
    fixture.assert_artifacts_bounded();
    fixture.assert_replacements_unchanged();
}

#[test]
fn post_install_failure_rolls_back_before_retry() {
    let mut fixture = RetainedFixture::fail_at(AppendStage::AfterRecordSync);
    let previous_cursor = fs::read(&fixture.moved_cursor).unwrap();
    set_post_install_hook(|| Err(std::io::Error::other(POST_INSTALL_FAULT).into()));

    assert!(fixture.reconcile().is_err());
    assert_eq!(fs::read(&fixture.moved_cursor).unwrap(), previous_cursor);
    fixture.reconcile().unwrap().unwrap();

    fixture.assert_exactly_once();
    fixture.assert_artifacts_bounded();
    fixture.assert_replacements_unchanged();
}
