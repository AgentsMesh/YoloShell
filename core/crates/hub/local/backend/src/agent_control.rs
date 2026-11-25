use crate::LocalBackend;
use uuid::Uuid;
use yolo_hub_agent_protocol::{
    AGENT_PROTOCOL_SCHEMA_VERSION, AgentAction, AgentControlFields, AgentControlRequest,
    AgentPromptReference, AgentPromptRequest, AgentSnapshot, CommandReceipt, FailureCode,
    ProtocolError,
};
use yolo_hub_identity::{AgentTarget, RuntimeTarget, TerminalAddress};
use yolo_hub_rpc_protocol::Reply;
use yolo_hub_terminal_protocol::{
    LeaseGrant, LeaseMode, LeaseRequest, PublicError, WriterPrincipal,
};

/// Submission half of an atomic prompt. Waiting is evaluated by the
/// dispatching layer, which owns the agent-wait sources.
pub struct PromptExecution {
    pub receipt: CommandReceipt,
    pub terminal: TerminalAddress,
    pub target: AgentTarget,
    pub runtime: RuntimeTarget,
    pub baseline: yolo_hub_agent_protocol::ObservationCursor,
}

impl LocalBackend {
    pub(crate) async fn control_agent(
        &self,
        request: &AgentControlRequest,
        grant: LeaseGrant,
    ) -> Result<Reply, PublicError> {
        if matches!(
            request.fields().action,
            yolo_hub_agent_protocol::AgentAction::Interrupt {}
                | yolo_hub_agent_protocol::AgentAction::SubmitPrompt { .. }
                | yolo_hub_agent_protocol::AgentAction::Respond { .. }
        ) {
            if let Some(projection) = &self.agents {
                // Admission validates the effect against one projection state,
                // so it keeps the lock; the sweep gate orders its observation
                // against catalogue sweeps.
                let _sweep = self.begin_observation().await;
                let mut projection = projection.lock().await;
                return self
                    .native_agent_control(request, grant, &mut projection)
                    .await;
            }
        }
        let snapshot = self
            .observe_agents()
            .await?
            .into_iter()
            .find(|snapshot| snapshot.cursor().target() == request.fields().target)
            .ok_or(PublicError::NotFound)?;
        Ok(Reply::AgentControl {
            receipt: reject_observation_only_control(request, grant, &snapshot),
        })
    }
}

impl LocalBackend {
    /// Acquire the writer, submit one prompt through the native control path,
    /// and release the writer. The returned baseline is the observation cursor
    /// seen before submission, so a turn wait only counts newer states.
    pub async fn prompt_agent(
        &self,
        request: &AgentPromptRequest,
    ) -> Result<PromptExecution, PublicError> {
        let fields = request.fields();
        let snapshot = self.prompt_snapshot(&fields.reference).await?;
        let target = snapshot.cursor().target();
        let control = AgentControlRequest::new(AgentControlFields {
            schema_version: AGENT_PROTOCOL_SCHEMA_VERSION,
            request_id: fields.request_id,
            target,
            expected_cursor: snapshot.cursor(),
            action: AgentAction::SubmitPrompt {
                prompt: fields.prompt.clone(),
            },
        })
        .map_err(|_| PublicError::InvalidRequest)?;
        control
            .validate_against(&snapshot)
            .map_err(|_| PublicError::Conflict)?;
        let runtime = snapshot.fields().terminal;
        let grant = self.prompt_grant(runtime, fields.take_control).await?;
        let reply = self.control_agent(&control, grant.clone()).await;
        let _ = self.release(grant).await;
        let Reply::AgentControl { receipt } = reply? else {
            return Err(PublicError::Internal);
        };
        Ok(PromptExecution {
            receipt,
            terminal: runtime.address,
            target,
            runtime,
            baseline: snapshot.cursor(),
        })
    }

    async fn prompt_snapshot(
        &self,
        reference: &AgentPromptReference,
    ) -> Result<AgentSnapshot, PublicError> {
        match reference {
            AgentPromptReference::Execution { target } => {
                match self.get_agent(*target).await? {
                    Reply::Agent { agent } => Ok(agent),
                    _ => Err(PublicError::Internal),
                }
            }
            AgentPromptReference::Terminal { address } => {
                if address.hub_id != self.hub {
                    return Err(PublicError::OriginMismatch);
                }
                self.observe_agents()
                    .await?
                    .into_iter()
                    .find(|agent| agent.fields().terminal.address == *address)
                    .ok_or(PublicError::NotFound)
            }
        }
    }

    async fn prompt_grant(
        &self,
        target: RuntimeTarget,
        take_control: bool,
    ) -> Result<LeaseGrant, PublicError> {
        let principal = WriterPrincipal {
            client_id: Uuid::new_v4(),
            controller_instance_id: Uuid::new_v4(),
        };
        let mode = if take_control {
            match self.observe_writer(target).await? {
                Some(lease) => LeaseMode::TakeControl {
                    observed_generation: lease.fields().generation,
                },
                None => LeaseMode::Acquire {},
            }
        } else {
            LeaseMode::Acquire {}
        };
        let request = LeaseRequest {
            request_id: Uuid::new_v4(),
            target,
            principal,
            mode,
        };
        match self.acquire(request).await? {
            Reply::Lease { grant } => Ok(grant),
            _ => Err(PublicError::Internal),
        }
    }
}

pub(crate) fn reject_observation_only_control(
    request: &AgentControlRequest,
    grant: LeaseGrant,
    snapshot: &AgentSnapshot,
) -> CommandReceipt {
    let reason = if grant.fields().target != snapshot.fields().terminal {
        FailureCode::PreconditionsChanged
    } else {
        match request.validate_against(snapshot) {
            Err(ProtocolError::UnsupportedControl) | Ok(()) => FailureCode::Unsupported,
            Err(_) => FailureCode::PreconditionsChanged,
        }
    };
    // This fallback covers unsupported actions and observation-only sources.
    // Target equality is not writer authentication. Positive native actions
    // use the separate Worker path with actual writer and source admission.
    CommandReceipt::rejected(request, reason)
}
