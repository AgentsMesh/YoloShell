use uuid::Uuid;
use yolo_hub_agent_protocol::*;
use yolo_hub_identity::*;

pub const STREAM: Uuid = Uuid::from_u128(7);

pub fn target() -> AgentTarget {
    AgentTarget {
        address: AgentAddress {
            hub_id: HubId::new(),
            agent_id: AgentId::new(),
        },
        execution_id: ExecutionId::new(),
    }
}

pub fn snapshot(
    target: AgentTarget,
    activity: Activity,
    lifecycle: Lifecycle,
    sequence: u64,
) -> AgentSnapshot {
    let fields = AgentSnapshotFields {
        schema_version: AGENT_PROTOCOL_SCHEMA_VERSION,
        cursor: ObservationCursor::new(target, STREAM, sequence).unwrap(),
        terminal: RuntimeTarget {
            address: TerminalAddress {
                hub_id: target.address.hub_id,
                terminal_id: TerminalId::new(),
            },
            runtime_epoch: RuntimeEpoch::new(),
        },
        provider: ObservedProvider::new("qwen_code".into(), None, Some("native".into())).unwrap(),
        working_directory: None,
        lifecycle,
        activity,
        reachability: Reachability::Reachable,
        freshness: Freshness::Current,
        source: ObservationSource::ProviderHook,
        observed_unix_ms: 1,
        recovery: RecoveryStatus::Available,
        capabilities: AgentCapabilities::UNSUPPORTED,
        interaction: None,
    };
    AgentSnapshot::new(fields).unwrap()
}

pub fn idle(target: AgentTarget) -> AgentSnapshot {
    snapshot(target, Activity::WaitingForInput, Lifecycle::Running {}, 2)
}

/// An idle snapshot whose control witness makes it safe to prompt.
pub fn promptable(target: AgentTarget) -> AgentSnapshot {
    let mut fields = idle(target).fields().clone();
    fields.capabilities = AgentCapabilities {
        interrupt: Capability::Available {},
        submit_prompt: Capability::Available {},
        respond_interaction: Capability::Unsupported {},
    };
    AgentSnapshot::new(fields).unwrap()
}

/// An approval snapshot with a typed response capability.
pub fn respondable(target: AgentTarget) -> AgentSnapshot {
    let mut fields = snapshot(
        target,
        Activity::WaitingForApproval,
        Lifecycle::Running {},
        3,
    )
    .fields()
    .clone();
    fields.capabilities = AgentCapabilities {
        interrupt: Capability::Available {},
        submit_prompt: Capability::Unsupported {},
        respond_interaction: Capability::Available {},
    };
    fields.interaction = Some(
        Interaction::with_prompt(
            InteractionReference {
                target,
                id: Uuid::new_v4(),
            },
            InteractionKind::Approval,
            vec![InteractionChoice {
                id: "allow".into(),
                label: "Allow".into(),
            }],
            "Bash: cargo test".into(),
        )
        .unwrap(),
    );
    AgentSnapshot::new(fields).unwrap()
}

pub fn request(
    targets: Vec<AgentTarget>,
    condition: AgentWaitCondition,
    after: Option<ObservationCursor>,
    wait_ms: u32,
) -> AgentWaitRequest {
    AgentWaitRequest {
        targets,
        runs: Vec::new(),
        condition,
        after,
        wait_ms,
    }
}

pub fn until_input() -> AgentWaitCondition {
    AgentWaitCondition::Activity {
        is: vec![Activity::WaitingForInput],
    }
}
