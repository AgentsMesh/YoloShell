use super::resume_consistency_support::*;

#[test]
fn independently_verified_bindings_cannot_inherit_another_execution_checkpoint() {
    for change in [
        BindingChange::ShellOwner,
        BindingChange::ProviderSession,
        BindingChange::WorkingDirectory,
        BindingChange::DataHome,
        BindingChange::TranscriptPath,
        BindingChange::DetectionTime,
    ] {
        let fixture = fixture();
        let original = original(&fixture);
        let before = stored(&fixture);
        let mut binding = original.binding.clone();
        change_binding(&mut binding, change);
        let candidate = rebuild(&fixture, &original, binding, true);
        let verified = candidate
            .clone()
            .with_verified_resume_identity(original.resume_boot_id().unwrap())
            .unwrap();
        verified.validate().unwrap();
        assert!(
            verified
                .recoverable_binding(fixture.runtime)
                .unwrap()
                .is_some()
        );
        assert!(
            candidate
                .inherit_verified_resume_identity(&original)
                .is_err(),
            "{change:?}"
        );
        assert_eq!(stored(&fixture), before, "{change:?}");
        assert!(
            original
                .recoverable_binding(fixture.runtime)
                .unwrap()
                .is_some()
        );
    }
}

#[test]
fn a_valid_older_observation_cannot_inherit_a_newer_resume_checkpoint() {
    let fixture = fixture();
    let initial = original(&fixture);
    let mut binding = initial.binding.clone();
    binding.updated_at_ms += CHECKPOINT_AGE;
    let checkpoint = rebuild(&fixture, &initial, binding, true)
        .with_verified_resume_identity(initial.resume_boot_id().unwrap())
        .unwrap();
    publish(&fixture, &checkpoint).unwrap();
    let before = stored(&fixture);
    let mut older = checkpoint.binding.clone();
    older.updated_at_ms -= TIMESTAMP_STEP;
    assert!(older.updated_at_ms >= older.detected_at_ms);
    let candidate = rebuild(&fixture, &checkpoint, older, true);
    let verified = candidate
        .clone()
        .with_verified_resume_identity(checkpoint.resume_boot_id().unwrap())
        .unwrap();
    verified.validate().unwrap();
    verified.verify_resume_evidence().unwrap();
    assert!(
        candidate
            .inherit_verified_resume_identity(&checkpoint)
            .is_err()
    );
    assert_eq!(stored(&fixture), before);
}

#[test]
fn capture_rejects_a_valid_transcript_for_a_different_provider_session() {
    let fixture = fixture();
    let original = original(&fixture);
    let before = stored(&fixture);
    let mut other = original.binding.clone();
    change_binding(&mut other, BindingChange::ProviderSession);
    let independent = rebuild(&fixture, &original, other.clone(), true)
        .with_verified_resume_identity(original.resume_boot_id().unwrap())
        .unwrap();
    independent.validate().unwrap();
    independent.verify_resume_evidence().unwrap();
    // Keep the other session's real transcript, but attach this execution's
    // binding identity. Neither valid constituent may authorize the combination.
    other.provider_session_id = original.binding.provider_session_id.clone();
    let mismatched = rebuild(&fixture, &original, other, true);
    mismatched.validate().unwrap();
    assert!(
        mismatched
            .with_verified_resume_identity(original.resume_boot_id().unwrap(),)
            .is_err()
    );
    assert_eq!(stored(&fixture), before);
}
