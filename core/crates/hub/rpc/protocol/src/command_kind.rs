use crate::{Command, Method};
use uuid::Uuid;

impl Command {
    pub fn method(&self) -> Method {
        match self {
            Self::GetClientAccess {} => Method::GetClientAccess,
            Self::ConfigureClientAccess { .. } => Method::ConfigureClientAccess,
            Self::Hello {} => Method::Hello,
            Self::GetBuild {} => Method::GetBuild,
            Self::GetSshProfiles {} => Method::GetSshProfiles,
            Self::ConfigureSshProfiles { .. } => Method::ConfigureSshProfiles,
            Self::ConnectSshProfile { .. } => Method::ConnectSshProfile,
            Self::RemoteSetup { .. } => Method::RemoteSetup,
            Self::InspectRemoteSetup { .. } => Method::InspectRemoteSetup,
            Self::List { .. } => Method::List,
            Self::Get { .. } => Method::Get,
            Self::Create { .. } => Method::Create,
            Self::TerminalRun { .. } => Method::TerminalRun,
            Self::AgentStart { .. } => Method::AgentStart,
            Self::AgentStop { .. } => Method::AgentStop,
            Self::AgentPrompt { .. } => Method::AgentPrompt,
            Self::CreateWorkspace { .. } => Method::CreateWorkspace,
            Self::UpdateWorkspace { .. } => Method::UpdateWorkspace,
            Self::GetWorkspace { .. } => Method::GetWorkspace,
            Self::ListWorkspaces { .. } => Method::ListWorkspaces,
            Self::CreateWorktree { .. } => Method::CreateWorktree,
            Self::RemoveWorktree { .. } => Method::RemoveWorktree,
            Self::ListWorktrees { .. } => Method::ListWorktrees,
            Self::GetAgentRun { .. } => Method::GetAgentRun,
            Self::ListAgentRuns { .. } => Method::ListAgentRuns,
            Self::UpdateAgentRun { .. } => Method::UpdateAgentRun,
            Self::QueryCreation { .. } => Method::QueryCreation,
            Self::Finalize { .. } => Method::Finalize,
            Self::Lease { .. } => Method::Lease,
            Self::Renew { .. } => Method::Renew,
            Self::Release { .. } => Method::Release,
            Self::Input { .. } => Method::Input,
            Self::Resize { .. } => Method::Resize,
            Self::Terminal { .. } => Method::Terminal,
            Self::Attach { .. } => Method::Attach,
            Self::ReadTerminalText { .. } => Method::ReadTerminalText,
            Self::ReadTerminalHistory { .. } => Method::ReadTerminalHistory,
            Self::ObserveLease { .. } => Method::ObserveLease,
            Self::ListAgents { .. } => Method::ListAgents,
            Self::WaitAgents { .. } => Method::WaitAgents,
            Self::GetAgent { .. } => Method::GetAgent,
            Self::GetCurrentAgent { .. } => Method::GetCurrentAgent,
            Self::ListHubs { .. } => Method::ListHubs,
            Self::ListProviders {} => Method::ListProviders,
            Self::ExportAudit { .. } => Method::ExportAudit,
            Self::CleanupTerminate { .. } => Method::CleanupTerminate,
            Self::CleanupArchive { .. } => Method::CleanupArchive,
            Self::GetConfiguration {} => Method::GetConfiguration,
            Self::Configure { .. } => Method::Configure,
            Self::MintInvite { .. } => Method::MintInvite,
            Self::ListFileRoots { .. } => Method::ListFileRoots,
            Self::ReadFile { .. } => Method::ReadFile,
            Self::AgentControl { .. } => Method::AgentControl,
            Self::GetPlatformSnapshot { .. } => Method::GetPlatformSnapshot,
            Self::ListPlatformEvents { .. } => Method::ListPlatformEvents,
            Self::SubscribeEvents { .. } => Method::SubscribeEvents,
            Self::ListLayouts {} => Method::ListLayouts,
            Self::GetLayout { .. } => Method::GetLayout,
            Self::UpdateLayout { .. } => Method::UpdateLayout,
            Self::ResolveConversation { .. } => Method::ResolveConversation,
            Self::ReadConversation { .. } => Method::ReadConversation,
            Self::WatchConversation { .. } => Method::WatchConversation,
        }
    }
    pub fn mutates(&self) -> bool {
        if let Self::RemoteSetup { call } = self {
            return call.mutates();
        }
        self.method().mutates()
    }

    /// Commands whose reply changes the terminal/agent catalogue projection.
    /// Terminal I/O and metadata-only mutations keep the cached catalogue.
    pub fn affects_catalogue(&self) -> bool {
        matches!(
            self.method(),
            Method::Create
                | Method::TerminalRun
                | Method::AgentStart
                | Method::AgentStop
                | Method::Finalize
                | Method::Terminal
                | Method::CleanupTerminate
                | Method::CleanupArchive
        )
    }

    pub fn request_id(&self) -> Option<Uuid> {
        match self {
            Self::ConfigureClientAccess { change } => Some(change.request_id),
            Self::Create { request } => Some(request.fields().request_id),
            Self::Lease { request } => Some(request.request_id),
            Self::Input { request } => Some(request.fields().request_id),
            Self::Resize { request } => Some(request.request_id),
            Self::Terminal { command } => Some(command.request_id),
            Self::Configure { change } => Some(change.request_id),
            Self::ConfigureSshProfiles { change } => Some(change.request_id),
            Self::RemoteSetup { call } => Some(call.request.request.fields().request_id),
            Self::AgentControl { request, .. } => Some(request.fields().request_id),
            Self::UpdateAgentRun { change } => Some(change.fields().request_id),
            Self::CreateWorktree { request } => Some(request.fields().request_id),
            Self::RemoveWorktree { change } => Some(change.fields().request_id),
            Self::UpdateLayout { change } => Some(change.request_id),
            _ => None,
        }
    }
}
