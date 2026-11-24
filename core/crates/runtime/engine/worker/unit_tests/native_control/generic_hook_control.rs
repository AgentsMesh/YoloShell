use super::generic_source::NativeSource;
use crate::agent::{AgentKind, AgentState};

#[test]
fn generic_source_builds_a_witness_for_the_published_binding() {
    let source = NativeSource::new();
    source
        .with(None, |binding, witness, verify| {
            assert_eq!(binding.kind, AgentKind::Other(yolo_qwen::qwen::constants::KIND.into()));
            assert_eq!(binding.state, AgentState::WaitingForInput);
            assert_eq!(witness.fields().kind, binding.kind);
            verify()
        })
        .unwrap();
}

#[test]
fn generic_source_rejects_a_revoked_authority() {
    let source = NativeSource::new();
    source.revoke();
    assert!(source.with(None, |_, _, _| Ok(())).is_err());
}

#[test]
fn generic_source_rejects_a_stale_witness_after_activity_changes() {
    let mut source = NativeSource::new();
    let stale = source
        .with(None, |_, witness, _| Ok(witness.clone()))
        .unwrap();
    source.change_activity();
    assert!(source.with(Some(&stale), |_, _, _| Ok(())).is_err());
}

#[test]
fn generic_source_rejects_removed_evidence() {
    let source = NativeSource::new();
    let evidence = std::path::Path::new(&source.record.binding.evidence_path);
    std::fs::remove_file(evidence).unwrap();
    assert!(source.with(None, |_, _, _| Ok(())).is_err());
}
