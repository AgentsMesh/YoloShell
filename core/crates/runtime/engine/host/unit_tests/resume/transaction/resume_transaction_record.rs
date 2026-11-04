use crate::host::tests::*;
use super::*;

const RESUME_UNKNOWN_FIELD: &str = "unexpected";

#[test]
fn durable_resume_transaction_is_strict_and_digest_bound() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let manifest = fixture_manifest(directory.path());
    let launch = shell_resume_launch(&host, &manifest);
    let transaction = shell_resume_transaction(Uuid::new_v4(), &launch);
    transaction.validate_for(manifest.terminal_id).unwrap();

    let mut encoded = serde_json::to_value(&transaction).unwrap();
    encoded
        .as_object_mut()
        .unwrap()
        .insert(RESUME_UNKNOWN_FIELD.into(), serde_json::Value::Null);
    assert!(serde_json::from_value::<ResumeTransaction>(encoded).is_err());

    let mut wrong_digest = transaction.clone();
    wrong_digest.launch_digest.replace_range(..1, "0");
    if wrong_digest.launch_digest == transaction.launch_digest {
        wrong_digest.launch_digest.replace_range(..1, "1");
    }
    assert!(wrong_digest.validate_for(manifest.terminal_id).is_err());

    let mut changed_launch = transaction.clone();
    changed_launch.launch.cols = changed_launch.launch.cols.saturating_add(1);
    assert!(changed_launch.validate_for(manifest.terminal_id).is_err());
}

#[test]
fn durable_resume_transaction_rejects_wrong_or_ambiguous_identities() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let manifest = fixture_manifest(directory.path());
    let launch = shell_resume_launch(&host, &manifest);
    let transaction = shell_resume_transaction(Uuid::new_v4(), &launch);

    let mut wrong_session = transaction.clone();
    wrong_session.terminal_id = Uuid::new_v4();
    assert!(wrong_session.validate_for(manifest.terminal_id).is_err());
    let mut wrong_previous = transaction.clone();
    wrong_previous.previous_runtime_epoch = transaction.planned_runtime_epoch;
    assert!(wrong_previous.validate_for(manifest.terminal_id).is_err());
    let mut changed_previous = transaction.clone();
    changed_previous.previous_runtime_epoch = Uuid::new_v4();
    assert!(changed_previous.validate_for(manifest.terminal_id).is_err());
    let mut changed_intent = transaction.clone();
    changed_intent.intent = ResumeIntent::Automatic;
    assert!(changed_intent.validate_for(manifest.terminal_id).is_err());
    let mut wrong_worker = transaction.clone();
    wrong_worker.worker_id = transaction.planned_runtime_epoch;
    assert!(wrong_worker.validate_for(manifest.terminal_id).is_err());
    let mut wrong_schema = transaction.clone();
    wrong_schema.schema_version += 1;
    assert!(wrong_schema.validate_for(manifest.terminal_id).is_err());
}

#[test]
fn durable_resume_explicit_exited_agent_target_never_degrades_to_shell() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let manifest = fixture_manifest(directory.path());
    let binding = host_binding(manifest.terminal_id);
    assert_eq!(
        ResumeIntent::Explicit.policy(&binding),
        ResumePolicy::Provider
    );
    let selected = ResumeBinding::Provider(binding.clone());
    let launch = host
        .prepare_worker_launch(
            &manifest,
            selected.launch_command(&manifest),
            Some((&binding, ResumeIntent::Explicit)),
            None,
        )
        .unwrap();
    let transaction = ResumeTransaction::new(
        Uuid::new_v4(),
        ResumeIntent::Explicit,
        ResumeTarget::from_binding(&selected),
        &launch,
    )
    .unwrap();
    assert!(matches!(
        transaction.target,
        ResumeTarget::Provider {
            ref agent_kind,
            ref provider_session_id,
            ..
        } if agent_kind == &crate::agent::AgentKind::Codex
            && provider_session_id == &binding.provider_session_id
    ));
    assert!(transaction.launch.initial_input.is_some());
    assert_eq!(transaction.target.completed_state(), RecoveryState::Resumed);

    let mut mismatched = transaction.clone();
    if let ResumeTarget::Provider {
        provider_session_id,
        binding,
        ..
    } = &mut mismatched.target
    {
        binding.provider_session_id.push('x');
        *provider_session_id = binding.provider_session_id.clone();
    }
    mismatched.transaction_digest = resume_transaction_digest(&mismatched).unwrap();
    mismatched.validate_for(manifest.terminal_id).unwrap();
    assert!(
        host.validate_transaction_launch(&manifest, &mismatched, &mismatched.launch)
            .is_err()
    );
}
