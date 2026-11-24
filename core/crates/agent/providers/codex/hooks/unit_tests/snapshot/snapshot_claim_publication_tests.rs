use super::snapshot_test_support::*;
use crate::codex_hooks::snapshot_error::CodexSnapshotFenceError;

#[test]
fn failed_publication_releases_the_claim_lock_without_installing_the_next_cursor() {
    let (_fixture, mut cursor) = SnapshotFixture::populated();

    let error = cursor
        .claim(&snapshot())
        .unwrap()
        .publish_with(|proof| {
            proof.linearize_publication()?;
            Err(anyhow::anyhow!(
                crate::codex_hooks::constants::errors::SNAPSHOT_CORRUPT
            ))
        })
        .unwrap_err();

    assert!(
        error
            .to_string()
            .contains(crate::codex_hooks::constants::errors::SNAPSHOT_CORRUPT)
    );
    assert_snapshot_changed(cursor.prove_current().err().unwrap());
}

#[test]
fn successful_publication_installs_the_next_cursor_after_proof_validation() {
    let (_fixture, mut cursor) = SnapshotFixture::populated();

    let token = cursor
        .claim(&snapshot())
        .unwrap()
        .publish_with(|proof| proof.linearize_publication())
        .unwrap();

    assert!(cursor.is_claimed_by(&token));
    cursor.prove_current().unwrap();
}

fn assert_snapshot_changed(error: anyhow::Error) {
    assert!(matches!(
        error.downcast_ref::<CodexSnapshotFenceError>(),
        Some(CodexSnapshotFenceError::SnapshotChanged)
    ));
}
