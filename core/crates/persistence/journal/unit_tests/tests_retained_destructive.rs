use super::tests_retained_hooks::set_post_install_hook;
use super::tests_retained_support::{RetainedFixture, saved_path};
use super::tests_security_support::{
    REPLACEMENT_BYTES, replace_target, temporary_path, write_private,
};
use super::*;
use std::fs;
use std::sync::{Arc, Mutex};

#[test]
fn staging_swap_after_final_verify_preserves_the_old_cursor() {
    let mut fixture = RetainedFixture::fail_at(AppendStage::AfterRecordSync);
    let previous_cursor = fs::read(&fixture.moved_cursor).unwrap();
    let parent = fixture.moved_journal.parent().unwrap().to_path_buf();
    crate::paths::test_support::set_retained_rename_hook(move || {
        let staging = temporary_path(
            &parent,
            crate::paths::test_support::retired_artifact_prefix(),
        );
        replace_target(&staging, &saved_path(&staging));
        Ok(())
    });

    assert!(fixture.reconcile().is_err());

    assert_eq!(fs::read(&fixture.moved_cursor).unwrap(), REPLACEMENT_BYTES);
    fixture.assert_retired_cursor(&previous_cursor);
    fixture.assert_artifacts_bounded();
    fixture.assert_replacements_unchanged();
}

#[test]
fn rollback_restore_eexist_preserves_both_cursor_capabilities() {
    let mut fixture = RetainedFixture::fail_at(AppendStage::AfterRecordSync);
    let previous_cursor = fs::read(&fixture.moved_cursor).unwrap();
    let cursor = fixture.moved_cursor.clone();
    let journal = fixture.moved_journal.clone();
    let saved_journal = saved_path(&journal);
    let saved_for_hook = saved_journal.clone();
    set_post_install_hook(move || {
        replace_target(&journal, &saved_for_hook);
        Ok(())
    });
    crate::paths::test_support::set_retained_restore_hook(move || {
        write_private(&cursor, REPLACEMENT_BYTES);
        Ok(())
    });

    assert!(fixture.reconcile().is_err());

    assert_eq!(fs::read(&fixture.moved_cursor).unwrap(), REPLACEMENT_BYTES);
    fixture.assert_retired_cursor(&previous_cursor);
    let staged = fixture.artifact(crate::paths::test_support::retired_artifact_prefix());
    assert_ne!(fs::read(staged).unwrap(), REPLACEMENT_BYTES);
    fixture.assert_artifacts_bounded();
    fixture.assert_replacements_unchanged();
}

#[test]
fn cleanup_name_replacement_is_never_truncated_or_unlinked() {
    let mut fixture = RetainedFixture::fail_at(AppendStage::AfterRecordSync);
    let parent = fixture.moved_journal.parent().unwrap().to_path_buf();
    let observed = Arc::new(Mutex::new(None));
    let captured = Arc::clone(&observed);
    crate::paths::test_support::set_retained_cleanup_recycle_hook(move || {
        let retired = temporary_path(
            &parent,
            crate::paths::test_support::retained_isolation_artifact_prefix(),
        );
        replace_target(&retired, &saved_path(&retired));
        *captured.lock().unwrap() = Some(retired);
        Ok(())
    });

    assert!(fixture.reconcile().is_err());
    let replacement = observed.lock().unwrap().take().unwrap();
    assert_eq!(fs::read(&replacement).unwrap(), REPLACEMENT_BYTES);
    let saved = saved_path(&replacement);
    fs::remove_file(&replacement).unwrap();
    fs::rename(saved, replacement).unwrap();
    fixture.reconcile().unwrap().unwrap();

    fixture.assert_exactly_once();
    fixture.assert_artifacts_bounded();
    fixture.assert_replacements_unchanged();
}
