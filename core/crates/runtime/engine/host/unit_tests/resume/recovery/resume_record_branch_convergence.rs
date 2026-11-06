use crate::host::tests::*;
use super::*;

const RESUME_SCHEMA_INCREMENT: u16 = 1;
const RESUME_LAUNCH_COLUMN_INCREMENT: u16 = 1;
const RESUME_PADDED_SESSION: &str = " padded-agent-session ";
const RESUME_REPLACEMENT_SESSION: &str = "replacement-agent-session";
const RESUME_DIGEST_CHARACTER: char = '0';

#[test]
fn convergence_provider_resume_target_rejects_each_independent_divergence() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let manifest = fixture_manifest(directory.path());
    let (_, _, transaction) = provider_resume_fixture(&host, &manifest);

    for mismatch in [
        ProviderTargetMismatch::EmptySession,
        ProviderTargetMismatch::PaddedSession,
        ProviderTargetMismatch::YoloSession,
        ProviderTargetMismatch::Kind,
        ProviderTargetMismatch::BindingSession,
        ProviderTargetMismatch::AutomaticExited,
    ] {
        let mut invalid = transaction.clone();
        mismatch.apply(&mut invalid);
        resign_resume_transaction(&mut invalid);
        assert!(invalid.validate_for(manifest.terminal_id).is_err());
    }
}

#[test]
fn convergence_resume_transaction_rejects_each_scalar_and_digest_boundary() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let manifest = fixture_manifest(directory.path());
    let launch = shell_resume_launch(&host, &manifest);
    let transaction = shell_resume_transaction(Uuid::new_v4(), &launch);

    let mut invalid = transaction.clone();
    invalid.terminal_id = Uuid::nil();
    assert!(invalid.validate_for(Uuid::nil()).is_err());
    invalid = transaction.clone();
    invalid.previous_runtime_epoch = Uuid::nil();
    assert!(invalid.validate_for(manifest.terminal_id).is_err());
    invalid = transaction.clone();
    invalid.planned_runtime_epoch = Uuid::nil();
    assert!(invalid.validate_for(manifest.terminal_id).is_err());
    invalid = transaction.clone();
    invalid.worker_id = Uuid::nil();
    assert!(invalid.validate_for(manifest.terminal_id).is_err());
    invalid = transaction.clone();
    invalid.created_at_ms = 0;
    assert!(invalid.validate_for(manifest.terminal_id).is_err());
    invalid = transaction.clone();
    invalid.launch_digest.clear();
    assert!(invalid.validate_for(manifest.terminal_id).is_err());
    invalid = transaction.clone();
    invalid.transaction_digest.clear();
    assert!(invalid.validate_for(manifest.terminal_id).is_err());
    invalid = transaction;
    invalid.transaction_digest =
        std::iter::repeat_n(RESUME_DIGEST_CHARACTER, SHA256_HEX_LENGTH).collect();
    assert!(invalid.validate_for(manifest.terminal_id).is_err());
}

#[test]
fn convergence_resume_transaction_rejects_each_launch_contract_boundary() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let manifest = fixture_manifest(directory.path());
    let launch = shell_resume_launch(&host, &manifest);
    let transaction = shell_resume_transaction(Uuid::new_v4(), &launch);

    for mismatch in [
        ResumeLaunchContractMismatch::Schema,
        ResumeLaunchContractMismatch::ControllerGeneration,
        ResumeLaunchContractMismatch::AuthToken,
    ] {
        let mut invalid = transaction.clone();
        mismatch.apply(&mut invalid.launch);
        resign_resume_transaction(&mut invalid);
        assert!(invalid.validate_for(manifest.terminal_id).is_err());
    }
}

#[test]
fn convergence_resume_launch_validation_fences_each_external_identity() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let manifest = fixture_manifest(directory.path());
    let launch = shell_resume_launch(&host, &manifest);
    let transaction = shell_resume_transaction(Uuid::new_v4(), &launch);

    for mismatch in [
        ExternalResumeLaunchMismatch::Session,
        ExternalResumeLaunchMismatch::RuntimeEpoch,
        ExternalResumeLaunchMismatch::Worker,
        ExternalResumeLaunchMismatch::Payload,
    ] {
        let mut candidate = launch.clone();
        mismatch.apply(&mut candidate);
        assert!(transaction.validate_launch(&candidate).is_err());
    }
}

#[derive(Clone, Copy)]
enum ProviderTargetMismatch {
    EmptySession,
    PaddedSession,
    YoloSession,
    Kind,
    BindingSession,
    AutomaticExited,
}

impl ProviderTargetMismatch {
    fn apply(self, transaction: &mut ResumeTransaction) {
        if matches!(self, Self::AutomaticExited) {
            transaction.intent = ResumeIntent::Automatic;
            return;
        }
        let ResumeTarget::Provider {
            agent_kind,
            provider_session_id,
            binding,
        } = &mut transaction.target
        else {
            panic!("provider transaction fixture changed")
        };
        match self {
            Self::EmptySession => provider_session_id.clear(),
            Self::PaddedSession => *provider_session_id = RESUME_PADDED_SESSION.into(),
            Self::YoloSession => binding.terminal_id = Uuid::new_v4(),
            Self::Kind => *agent_kind = crate::agent::AgentKind::ClaudeCode,
            Self::BindingSession => {
                binding.provider_session_id = RESUME_REPLACEMENT_SESSION.into();
            }
            Self::AutomaticExited => unreachable!(),
        }
    }
}

#[derive(Clone, Copy)]
enum ResumeLaunchContractMismatch {
    Schema,
    ControllerGeneration,
    AuthToken,
}

impl ResumeLaunchContractMismatch {
    fn apply(self, launch: &mut WorkerLaunchSpec) {
        match self {
            Self::Schema => launch.schema_version += RESUME_SCHEMA_INCREMENT,
            Self::ControllerGeneration => launch.controller_generation = 0,
            Self::AuthToken => launch.auth_token.clear(),
        }
    }
}

#[derive(Clone, Copy)]
enum ExternalResumeLaunchMismatch {
    Session,
    RuntimeEpoch,
    Worker,
    Payload,
}

impl ExternalResumeLaunchMismatch {
    fn apply(self, launch: &mut WorkerLaunchSpec) {
        match self {
            Self::Session => launch.terminal_id = Uuid::new_v4(),
            Self::RuntimeEpoch => launch.runtime_epoch = Uuid::new_v4(),
            Self::Worker => launch.worker_id = Uuid::new_v4(),
            Self::Payload => launch.cols += RESUME_LAUNCH_COLUMN_INCREMENT,
        }
    }
}

fn resign_resume_transaction(transaction: &mut ResumeTransaction) {
    transaction.launch_digest = resume_launch_digest(&transaction.launch).unwrap();
    transaction.transaction_digest = resume_transaction_digest(transaction).unwrap();
}
