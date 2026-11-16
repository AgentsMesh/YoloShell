use yolo_conversation_protocol::{
    ConversationAddress, ConversationAvailability, ConversationBinding, ConversationDescriptor,
    ConversationEntry, ConversationPage, ConversationPart, ConversationProvider,
    ConversationReadResult, ConversationRole, ConversationSource,
};
use yolo_crypto::Sha256Digest;
use yolo_hub_agent_protocol::{
    AGENT_PROTOCOL_SCHEMA_VERSION, Activity, AgentCapabilities, AgentSnapshot, AgentSnapshotFields,
    Capability, CommandOutcome, CommandReceipt, Freshness, Lifecycle, ObservationCursor,
    ObservationSource, ObservedProvider, Reachability, RecoveryStatus,
};
use yolo_hub_identity::{
    AgentAddress, AgentId, AgentTarget, ExecutionId, HubId, RuntimeEpoch, RuntimeTarget,
    TerminalAddress, TerminalId,
};
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::{
    LeaseGrant, LeaseGrantFields, RuntimeStatus, RuntimeSummary, SecretCapability, TerminalSummary,
    TerminalSummaryFields,
};

pub const PROVIDER_KIND: &str = "probe";

pub struct Fixture {
    pub hub: HubId,
    pub terminal_id: TerminalId,
    pub agent_id: AgentId,
    pub execution_id: ExecutionId,
    pub runtime_epoch: RuntimeEpoch,
}

impl Fixture {
    pub fn new() -> Self {
        Self {
            hub: HubId::new(),
            terminal_id: TerminalId::new(),
            agent_id: AgentId::new(),
            execution_id: ExecutionId::new(),
            runtime_epoch: RuntimeEpoch::new(),
        }
    }

    pub fn target(&self) -> AgentTarget {
        AgentTarget {
            address: AgentAddress {
                hub_id: self.hub,
                agent_id: self.agent_id,
            },
            execution_id: self.execution_id,
        }
    }

    pub fn runtime(&self) -> RuntimeTarget {
        RuntimeTarget {
            address: TerminalAddress {
                hub_id: self.hub,
                terminal_id: self.terminal_id,
            },
            runtime_epoch: self.runtime_epoch,
        }
    }

    pub fn terminal(&self, name: &str) -> TerminalSummary {
        TerminalSummary::new(TerminalSummaryFields {
            address: self.runtime().address,
            name: name.into(),
            presentation: Default::default(),
            cwd: "/tmp".into(),
            archived: false,
            runtime: Some(RuntimeSummary {
                target: self.runtime(),
                status: RuntimeStatus::Running,
                output_sequence: 0,
                size: None,
                safe_termination: None,
            }),
            agent: Some(self.target()),
            recovery: None,
            workspace: None,
        })
        .unwrap()
    }

    pub fn agent(&self, activity: Activity, lifecycle: Lifecycle) -> AgentSnapshot {
        let exited = matches!(lifecycle, Lifecycle::Exited { .. });
        AgentSnapshot::new(AgentSnapshotFields {
            schema_version: AGENT_PROTOCOL_SCHEMA_VERSION,
            cursor: ObservationCursor::new(self.target(), uuid::Uuid::new_v4(), 1).unwrap(),
            terminal: self.runtime(),
            provider: ObservedProvider::new(PROVIDER_KIND.into(), None, None).unwrap(),
            working_directory: None,
            lifecycle,
            activity,
            reachability: Reachability::Reachable,
            freshness: Freshness::Current,
            source: ObservationSource::ProviderHook,
            observed_unix_ms: 1,
            recovery: RecoveryStatus::Available,
            capabilities: if exited {
                AgentCapabilities::UNSUPPORTED
            } else {
                AgentCapabilities {
                    interrupt: Capability::Available {},
                    submit_prompt: Capability::Available {},
                    respond_interaction: Capability::Unsupported {},
                }
            },
            interaction: None,
        })
        .unwrap()
    }

    pub fn descriptor(&self) -> ConversationDescriptor {
        ConversationDescriptor {
            address: ConversationAddress {
                hub_id: self.hub,
                conversation_id: Sha256Digest::repeated(9),
            },
            binding: ConversationBinding {
                agent: self.target(),
                runtime: self.runtime(),
            },
            provider: ConversationProvider::new(PROVIDER_KIND).unwrap(),
            provider_session_id: "session".into(),
            source: ConversationSource::NativeTranscript,
        }
    }

    pub fn page(&self, revision: u8, entries: Vec<ConversationEntry>) -> ConversationReadResult {
        ConversationReadResult::Page {
            page: ConversationPage {
                conversation: self.descriptor(),
                history_generation: Sha256Digest::repeated(1),
                revision: Sha256Digest::repeated(revision),
                availability: ConversationAvailability::Available,
                partial: Vec::new(),
                entries,
                next_cursor: None,
            },
        }
    }

    pub fn lease(&self, command: &Command) -> Reply {
        let Command::Lease { request } = command else {
            panic!("expected a lease command")
        };
        let grant = LeaseGrant::new(LeaseGrantFields {
            target: request.target,
            principal: request.principal,
            generation: 1,
            capability: SecretCapability::generate(),
            expires_at_ms: 4_000_000_000_000,
            ttl_ms: 300_000,
            renew_after_ms: 100_000,
        })
        .unwrap();
        Reply::Lease { grant }
    }

    pub fn receipt(&self, command: &Command, outcome: CommandOutcome) -> Reply {
        let Command::AgentControl { request, .. } = command else {
            panic!("expected an agent control command")
        };
        let mut receipt =
            CommandReceipt::accepted(request.fields().request_id, request.fields().target).unwrap();
        receipt.advance(outcome).unwrap();
        Reply::AgentControl { receipt }
    }
}

pub fn text_entry(id: u8, role: ConversationRole, text: &str) -> ConversationEntry {
    ConversationEntry {
        id: Sha256Digest::repeated(id),
        parent_id: None,
        native_id: None,
        timestamp: None,
        role,
        parts: vec![ConversationPart::Text { text: text.into() }],
    }
}
