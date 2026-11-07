use crate::host::tests::*;
use super::*;

const RESUME_BINDING_SCHEMA_INCREMENT: u16 = 1;

#[test]
fn convergence_binding_clear_without_provenance_rejects_an_existing_binding() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let manifest = fixture_manifest(directory.path());
    host.paths.ensure_session(manifest.terminal_id).unwrap();
    let binding = host_binding(manifest.terminal_id);
    let store = AgentBindingStore::new(host.paths.agent_binding(manifest.terminal_id));
    store.save(&binding).unwrap();

    assert!(
        host.clear_previous_binding(manifest.terminal_id, None)
            .is_err()
    );
    assert_eq!(read_json::<AgentBinding>(store.path()).unwrap(), binding);
}

#[test]
fn convergence_binding_marker_validation_requires_a_clear_transaction() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let manifest = fixture_manifest(directory.path());
    let clear = binding_clear_transaction(&host, &manifest, None);
    let marker = binding_clear_marker(&clear, ResumeBindingClearPhase::Cleared).unwrap();
    let preserve = shell_resume_transaction(Uuid::new_v4(), &shell_resume_launch(&host, &manifest));

    assert!(marker.validate_for(&preserve).is_err());
    assert!(marker.validate_rollover_to(&preserve).is_err());
}

#[test]
fn convergence_binding_marker_rollover_rejects_every_provenance_divergence() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let manifest = fixture_manifest(directory.path());
    let previous = Some(host_binding(manifest.terminal_id));
    let first = binding_clear_transaction(&host, &manifest, previous);
    let marker = binding_clear_marker(&first, ResumeBindingClearPhase::Cleared).unwrap();
    let current = ResumeTransaction::new(
        first.planned_runtime_epoch,
        ResumeIntent::Explicit,
        first.target.clone(),
        &shell_resume_launch(&host, &manifest),
    )
    .unwrap();
    marker.validate_rollover_to(&current).unwrap();

    for mismatch in [
        BindingRolloverMismatch::Schema,
        BindingRolloverMismatch::Session,
        BindingRolloverMismatch::PreviousEpoch,
        BindingRolloverMismatch::CurrentDigest,
        BindingRolloverMismatch::InvalidDigest,
        BindingRolloverMismatch::PreviousBinding,
        BindingRolloverMismatch::Phase,
    ] {
        let mut invalid = marker.clone();
        mismatch.apply(&mut invalid, &current);
        assert!(invalid.validate_rollover_to(&current).is_err());
    }
}

#[derive(Clone, Copy)]
enum BindingRolloverMismatch {
    Schema,
    Session,
    PreviousEpoch,
    CurrentDigest,
    InvalidDigest,
    PreviousBinding,
    Phase,
}

impl BindingRolloverMismatch {
    fn apply(self, marker: &mut ResumeBindingClearMarker, current: &ResumeTransaction) {
        match self {
            Self::Schema => marker.schema_version += RESUME_BINDING_SCHEMA_INCREMENT,
            Self::Session => marker.terminal_id = Uuid::new_v4(),
            Self::PreviousEpoch => marker.planned_runtime_epoch = Uuid::new_v4(),
            Self::CurrentDigest => {
                marker.transaction_digest = current.transaction_digest.clone();
            }
            Self::InvalidDigest => marker.transaction_digest.clear(),
            Self::PreviousBinding => marker.previous = None,
            Self::Phase => marker.phase = ResumeBindingClearPhase::Pending,
        }
    }
}
