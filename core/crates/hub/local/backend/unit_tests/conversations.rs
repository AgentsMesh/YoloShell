use super::fixtures::constants::INVALID_CONVERSATION_ENTRY_LIMIT;
use super::fixtures::{agent_control::host_view, target};
use crate::{LocalBackend, conversations::binding::same_native_run};
use yolo_agent_contract::{AgentBinding, AgentState};
use yolo_agent_control_contract::*;
use yolo_conversation_protocol::*;
use yolo_crypto::Sha256Digest;
use yolo_hub_identity::*;
use yolo_hub_rpc_protocol::Reply;

fn observed(
    binding: AgentBinding,
    worker: uuid::Uuid,
    boot: &str,
    epoch: uuid::Uuid,
) -> NativeControlObservation {
    let witness = NativeControlWitness::new(NativeControlWitnessFields {
        terminal_id: binding.terminal_id,
        runtime_epoch: epoch,
        worker_id: worker,
        boot_id: boot.into(),
        kind: binding.kind.clone(),
        process_pid: binding.process_pid.unwrap(),
        process_start_time: binding.process_start_time.unwrap(),
        binding_digest: binding_digest(&binding).unwrap(),
        authority_digest: authority_digest(&0).unwrap(),
    })
    .unwrap();
    NativeControlObservation::new(binding, witness, false).unwrap()
}

#[test]
fn read_fence_ignores_activity_but_rejects_every_evidence_and_process_identity_change() {
    let view = host_view();
    let original = view.agent.unwrap();
    let worker = uuid::Uuid::new_v4();
    let epoch = target().runtime_epoch.as_uuid();
    let before = observed(original.clone(), worker, "test-boot", epoch);
    let mut active = original.clone();
    active.state = AgentState::WaitingForInput;
    active.updated_at_ms += 1;
    assert!(same_native_run(
        &before,
        &observed(active, worker, "test-boot", epoch)
    ));
    let mutations: [fn(&mut AgentBinding); 8] = [
        |binding| binding.terminal_id = uuid::Uuid::new_v4(),
        |binding| binding.provider_session_id.push_str("-other"),
        |binding| binding.executable.push_str("-other"),
        |binding| binding.cwd.push_str("/other"),
        |binding| binding.evidence_path.push_str("/other"),
        |binding| binding.data_home.push_str("/other"),
        |binding| binding.process_pid = binding.process_pid.map(|pid| pid + 1),
        |binding| binding.process_start_time = binding.process_start_time.map(|time| time + 1),
    ];
    for change in mutations {
        let mut binding = original.clone();
        change(&mut binding);
        assert!(!same_native_run(
            &before,
            &observed(binding, worker, "test-boot", epoch)
        ));
    }
    for (worker, boot, epoch) in [
        (uuid::Uuid::new_v4(), "test-boot", epoch),
        (worker, "other-boot", epoch),
        (worker, "test-boot", uuid::Uuid::new_v4()),
    ] {
        assert!(!same_native_run(
            &before,
            &observed(original.clone(), worker, boot, epoch)
        ));
    }
}

#[tokio::test]
async fn unsupported_origins_invalid_limits_and_waits_never_contact_host() {
    let directory = tempfile::tempdir().unwrap();
    let home = directory.path().canonicalize().unwrap();
    let backend =
        LocalBackend::new(target().address.hub_id, &home, &home.join("missing.sock")).unwrap();
    let binding = ConversationBinding {
        agent: AgentTarget {
            address: AgentAddress {
                hub_id: backend.hub_id(),
                agent_id: AgentId::new(),
            },
            execution_id: ExecutionId::new(),
        },
        runtime: target(),
    };
    assert_eq!(
        backend.resolve_conversation(binding).await.unwrap(),
        Reply::ConversationUnavailable {
            error: ConversationError::UnsupportedProvider
        }
    );
    let mut foreign = binding;
    foreign.agent.address.hub_id = HubId::new();
    assert_eq!(
        backend.resolve_conversation(foreign).await.unwrap(),
        Reply::ConversationUnavailable {
            error: ConversationError::InvalidRequest
        }
    );
    foreign.runtime.address.hub_id = foreign.agent.address.hub_id;
    assert_eq!(
        backend.resolve_conversation(foreign).await.unwrap(),
        Reply::ConversationUnavailable {
            error: ConversationError::BindingChanged
        }
    );
    let mut request = ConversationReadRequest {
        address: ConversationAddress {
            hub_id: backend.hub_id(),
            conversation_id: Sha256Digest::repeated(1),
        },
        binding,
        cursor: None,
        limits: ConversationLimits::default(),
        since_revision: None,
        since_generation: None,
    };
    assert_eq!(
        backend
            .watch_conversation(&request, MAX_WATCH_WAIT_MS + 1)
            .await
            .unwrap(),
        Reply::ConversationUnavailable {
            error: ConversationError::InvalidRequest
        }
    );
    assert_eq!(
        backend.watch_conversation(&request, 0).await.unwrap(),
        Reply::ConversationUnavailable {
            error: ConversationError::UnsupportedProvider
        }
    );
    request.limits.entries = INVALID_CONVERSATION_ENTRY_LIMIT;
    assert_eq!(
        backend.read_conversation(&request).await.unwrap(),
        Reply::ConversationUnavailable {
            error: ConversationError::InvalidRequest
        }
    );
}
