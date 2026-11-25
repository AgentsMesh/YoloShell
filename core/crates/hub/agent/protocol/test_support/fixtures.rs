use uuid::Uuid;
use yolo_hub_agent_protocol::*;
use yolo_hub_identity::*;

#[path = "constants.rs"]
mod constants;
pub use constants::*;

pub fn target() -> AgentTarget {
    AgentTarget {
        address: AgentAddress {
            hub_id: HubId::new(),
            agent_id: AgentId::new(),
        },
        execution_id: ExecutionId::new(),
    }
}

pub fn fields() -> AgentSnapshotFields {
    let target = target();
    AgentSnapshotFields {
        schema_version: AGENT_PROTOCOL_SCHEMA_VERSION,
        cursor: ObservationCursor::new(target, Uuid::new_v4(), FIRST_OBSERVATION_SEQUENCE).unwrap(),
        terminal: RuntimeTarget {
            address: TerminalAddress {
                hub_id: target.address.hub_id,
                terminal_id: TerminalId::new(),
            },
            runtime_epoch: RuntimeEpoch::new(),
        },
        provider: ObservedProvider::new(
            PROVIDER_KIND.into(),
            Some(PROVIDER_VERSION.into()),
            Some(NATIVE_REFERENCE.into()),
        )
        .unwrap(),
        working_directory: None,
        lifecycle: Lifecycle::Running {},
        activity: Activity::Working,
        reachability: Reachability::Reachable,
        freshness: Freshness::Current,
        source: ObservationSource::ProviderHook,
        observed_unix_ms: OBSERVED_UNIX_MS,
        recovery: RecoveryStatus::Available,
        capabilities: AgentCapabilities {
            interrupt: Capability::Available {},
            submit_prompt: Capability::Unsupported {},
            respond_interaction: Capability::Unsupported {},
        },
        interaction: None,
    }
}

pub fn choice() -> InteractionChoice {
    InteractionChoice {
        id: CHOICE_ID.into(),
        label: CHOICE_LABEL.into(),
    }
}

pub fn interaction(target: AgentTarget, kind: InteractionKind) -> Interaction {
    Interaction::new(
        InteractionReference {
            target,
            id: Uuid::new_v4(),
        },
        kind,
        if kind == InteractionKind::Text {
            Vec::new()
        } else {
            vec![choice()]
        },
    )
    .unwrap()
}

pub fn waiting(kind: InteractionKind) -> AgentSnapshot {
    let mut fields = fields();
    fields.activity = if kind == InteractionKind::Approval {
        Activity::WaitingForApproval
    } else {
        Activity::WaitingForInput
    };
    fields.interaction = Some(interaction(fields.cursor.target(), kind));
    fields.capabilities.respond_interaction = Capability::Available {};
    AgentSnapshot::new(fields).unwrap()
}

pub fn request_fields(snapshot: &AgentSnapshot) -> AgentControlFields {
    AgentControlFields {
        schema_version: AGENT_PROTOCOL_SCHEMA_VERSION,
        request_id: Uuid::new_v4(),
        target: snapshot.cursor().target(),
        expected_cursor: snapshot.cursor(),
        action: AgentAction::Interrupt {},
    }
}

pub fn response_fields(snapshot: &AgentSnapshot) -> AgentControlFields {
    let mut request = request_fields(snapshot);
    let pending = snapshot.fields().interaction.as_ref().unwrap();
    let response = if pending.kind() == InteractionKind::Text {
        InteractionResponse::Text {
            text: TEXT_RESPONSE.into(),
        }
    } else {
        InteractionResponse::Choice {
            choice: CHOICE_ID.into(),
        }
    };
    request.action = AgentAction::Respond {
        interaction: pending.reference(),
        response,
    };
    request
}

pub fn round_trip<T>(value: &T) -> T
where
    T: serde::Serialize + serde::de::DeserializeOwned,
{
    serde_json::from_slice(&serde_json::to_vec(value).unwrap()).unwrap()
}
