use super::resume_consistency_support::*;
use crate::codex::authority::CodexAuthorityStatus;
use crate::{AgentRecoveryAvailability, AgentState};

#[test]
fn canonical_terminal_or_new_revision_cannot_replace_a_verified_resume_source() {
    for change in [
        SourceChange::Boot,
        SourceChange::TranscriptPath,
        SourceChange::DataHomePath,
        SourceChange::TranscriptObject,
        SourceChange::DirectoryObject,
        SourceChange::DataHomeObject,
    ] {
        for advance in [false, true] {
            let fixture = fixture();
            let original = original(&fixture);
            let before = stored(&fixture);
            let mut binding = original.binding.clone();
            change_source(&mut binding, change);
            let boot = if matches!(change, SourceChange::Boot) {
                OTHER_BOOT
            } else {
                original.resume_boot_id().unwrap()
            };
            let verified = rebuild(&fixture, &original, binding, advance)
                .with_verified_resume_identity(boot)
                .unwrap();
            verified.validate().unwrap();
            verified.verify_resume_evidence().unwrap();
            assert!(
                verified
                    .recoverable_binding(fixture.runtime)
                    .unwrap()
                    .is_some()
            );
            let mut candidate = verified;
            if !advance {
                candidate.status = CodexAuthorityStatus::Unavailable;
                candidate.binding.state = AgentState::Unknown;
                candidate.binding.recovery_availability = AgentRecoveryAvailability::Unavailable;
            }
            candidate.validate().unwrap();
            assert!(
                publish(&fixture, &candidate).is_err(),
                "{change:?} advance={advance}"
            );
            assert_eq!(stored(&fixture), before, "{change:?} advance={advance}");
        }
    }
}

#[test]
fn explicit_cwd_change_with_the_same_transcript_and_home_is_a_valid_successor() {
    let fixture = fixture();
    let previous = original(&fixture);
    let mut binding = previous.binding.clone();
    change_binding(&mut binding, BindingChange::WorkingDirectory);
    let next = rebuild(&fixture, &previous, binding, true)
        .with_verified_resume_identity(previous.resume_boot_id().unwrap())
        .unwrap();
    publish(&fixture, &next).unwrap();
    assert_eq!(original(&fixture), next);
    assert!(next.recoverable_binding(fixture.runtime).unwrap().is_some());
}
