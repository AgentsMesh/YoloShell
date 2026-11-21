use crate::Command;
use yolo_hub_identity::HubId;
use yolo_hub_management_protocol::FIRST_CONFIGURATION_REVISION;
use yolo_hub_terminal_protocol::PublicError;

impl Command {
    pub fn validate_origin(&self, origin: HubId) -> Result<(), PublicError> {
        let owner = match self {
            Self::ListLayouts {} => None,
            Self::GetLayout { address } => {
                if address.layout_id.is_nil() {
                    return Err(PublicError::InvalidRequest);
                }
                Some(address.hub_id)
            }
            Self::UpdateLayout { change } => {
                change.validate().map_err(|_| PublicError::InvalidRequest)?;
                Some(change.address.hub_id)
            }
            Self::ResolveConversation { binding } => {
                binding
                    .validate()
                    .map_err(|_| PublicError::InvalidRequest)?;
                Some(binding.agent.address.hub_id)
            }
            Self::ReadConversation { request } | Self::WatchConversation { request, .. } => {
                request
                    .validate()
                    .map_err(|_| PublicError::InvalidRequest)?;
                if let Self::WatchConversation { wait_ms, .. } = self {
                    if *wait_ms > yolo_conversation_protocol::MAX_WATCH_WAIT_MS {
                        return Err(PublicError::InvalidRequest);
                    }
                }
                Some(request.address.hub_id)
            }
            Self::GetClientAccess {} => None,
            Self::ConfigureClientAccess { change } => {
                change.validate().map_err(|_| PublicError::InvalidRequest)?;
                Some(change.configuration.entry_hub)
            }
            Self::InspectRemoteSetup { operation_id } => {
                if operation_id.is_nil() {
                    return Err(PublicError::InvalidRequest);
                }
                None
            }
            Self::RemoteSetup { call } => {
                call.validate().map_err(|_| PublicError::InvalidRequest)?;
                None
            }
            Self::ConnectSshProfile { .. } => None,
            Self::ConfigureSshProfiles { change } => {
                change.validate().map_err(|_| PublicError::InvalidRequest)?;
                None
            }
            Self::AgentControl { request, grant } => {
                let owner = request.fields().target.address.hub_id;
                if grant.fields().target.address.hub_id != owner {
                    return Err(PublicError::OriginMismatch);
                }
                Some(owner)
            }
            Self::AgentPrompt { request } => match &request.fields().reference {
                yolo_hub_agent_protocol::AgentPromptReference::Execution { target } => {
                    Some(target.address.hub_id)
                }
                yolo_hub_agent_protocol::AgentPromptReference::Terminal { address } => {
                    Some(address.hub_id)
                }
            },
            Self::Get { address } => Some(address.hub_id),
            Self::UpdateWorkspace { change } => Some(change.fields().address.hub_id),
            Self::GetWorkspace { request } => match &request.fields().selector {
                yolo_hub_workspace_protocol::WorkspaceSelector::Address(address) => {
                    Some(address.hub_id)
                }
                yolo_hub_workspace_protocol::WorkspaceSelector::Name(_) => None,
            },
            // Federated listings return cursors owned by peer Hubs.
            Self::ListWorkspaces { .. } => None,
            Self::UpdateAgentRun { change } => Some(change.fields().address.hub_id),
            Self::GetAgentRun { request } => match &request.fields().selector {
                yolo_hub_agent_run_protocol::AgentRunSelector::Address(address) => {
                    Some(address.hub_id)
                }
                yolo_hub_agent_run_protocol::AgentRunSelector::Name(_) => None,
            },
            Self::ListAgentRuns { .. } => None,
            Self::CreateWorktree { request } => Some(request.fields().parent.hub_id),
            Self::RemoveWorktree { change } => Some(change.fields().address.hub_id),
            Self::ListWorktrees { request } => Some(request.fields().parent.hub_id),
            Self::Finalize {
                creation_token,
                creation,
            } => {
                if creation_token.is_nil() {
                    return Err(PublicError::InvalidRequest);
                }
                Some(creation.address.hub_id)
            }
            Self::Lease { request } => {
                request.validate()?;
                Some(request.target.address.hub_id)
            }
            Self::Renew { grant } | Self::Release { grant } => {
                Some(grant.fields().target.address.hub_id)
            }
            Self::Input { request } => Some(request.fields().grant.fields().target.address.hub_id),
            Self::Resize { request } => {
                request.validate()?;
                Some(request.grant.fields().target.address.hub_id)
            }
            Self::Terminal { command } => {
                command.validate()?;
                Some(command.selection.address.hub_id)
            }
            Self::Attach { request } => Some(request.target().address.hub_id),
            Self::ReadTerminalText { request } => Some(request.fields().target.address.hub_id),
            Self::ReadTerminalHistory { request } => {
                Some(request.fields().target.address.hub_id)
            }
            Self::ObserveLease { target } => Some(target.address.hub_id),
            Self::GetAgent { target } => Some(target.address.hub_id),
            Self::WaitAgents { request } => {
                request
                    .validate()
                    .map_err(|_| PublicError::InvalidRequest)?;
                None
            }
            Self::GetCurrentAgent { address } => Some(address.hub_id),
            Self::AgentStop { request } => match &request.fields().selector {
                yolo_hub_agent_run_protocol::AgentRunSelector::Address(address) => {
                    Some(address.hub_id)
                }
                yolo_hub_agent_run_protocol::AgentRunSelector::Name(_) => None,
            },
            Self::ReadFile { request } => Some(request.fields().target.address.hub_id),
            Self::CleanupTerminate { creation, .. } | Self::CleanupArchive { creation } => {
                Some(creation.address.hub_id)
            }
            Self::Configure { change } => {
                if change.request_id.is_nil()
                    || change.expected_revision < FIRST_CONFIGURATION_REVISION
                {
                    return Err(PublicError::InvalidRequest);
                }
                None
            }
            Self::Hello {}
            | Self::GetBuild {}
            | Self::GetSshProfiles {}
            | Self::List { .. }
            | Self::Create { .. }
            | Self::TerminalRun { .. }
            | Self::AgentStart { .. }
            | Self::CreateWorkspace { .. }
            | Self::QueryCreation { .. }
            | Self::ListAgents { .. }
            | Self::ListHubs { .. }
            | Self::ListProviders {} => None,
            Self::ExportAudit { limit, .. }
                if *limit == 0 || *limit > crate::MAX_AUDIT_EXPORT_LIMIT =>
            {
                return Err(PublicError::InvalidRequest);
            }
            Self::ExportAudit { .. } => None,
            Self::GetPlatformSnapshot { .. }
            | Self::ListPlatformEvents { .. }
            | Self::SubscribeEvents { .. }
            | Self::GetConfiguration {}
            | Self::ListConsumers {} => None,
            Self::ListFileRoots { .. } => None,
        };
        if owner.is_some_and(|owner| owner != origin) {
            return Err(PublicError::OriginMismatch);
        }
        Ok(())
    }
    pub fn requires_read_reply_admission(&self) -> bool {
        self.method().operation().requires_read_reply_admission()
    }
    pub fn local_only(&self) -> bool {
        self.method().operation().access == crate::Access::LocalManagement
    }

    /// Local-management operations are never forwarded across a peer boundary.
    pub fn peer_forbidden(&self) -> bool {
        self.local_only()
    }
}
