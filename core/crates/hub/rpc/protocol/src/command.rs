use serde::{Deserialize, Serialize};
use uuid::Uuid;
use yolo_hub_agent_protocol::{AgentControlRequest, AgentStartRequest};
use yolo_hub_agent_run_protocol::{
    GetAgentRunRequest, ListAgentRunsRequest, UpdateAgentRunRequest,
};
use yolo_hub_file_protocol::ReadFileRequest;
use yolo_hub_identity::{
    AgentAddress, AgentTarget, FileRootId, HubId, RuntimeTarget, TerminalAddress,
};
use yolo_hub_management_protocol::ConfigurationChange;
use yolo_hub_terminal_protocol::{
    AttachRequest, CreateShellRequest, CreateShellResult, CreationQueryRequest, InputRequest,
    LeaseGrant, LeaseRequest, ReadTerminalHistoryRequest, ReadTerminalTextRequest, ResizeRequest,
    TerminalCommand, TerminalRunRequest,
};
use yolo_hub_workspace_protocol::{
    CreateWorkspaceRequest, CreateWorktreeRequest, GetWorkspaceRequest, ListWorkspacesRequest,
    ListWorktreesRequest, RemoveWorktreeRequest, UpdateWorkspaceRequest,
};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "method", rename_all = "snake_case", deny_unknown_fields)]
pub enum Command {
    GetClientAccess {},
    ConfigureClientAccess { change: yolo_hub_client_access_protocol::ClientAccessChange },
    Hello {},
    GetBuild {},
    GetSshProfiles {},
    ConfigureSshProfiles { change: yolo_hub_ssh_profile_protocol::SshProfileChange },
    ConnectSshProfile {
        profile: yolo_ssh_profile_protocol::SshProfileRef,
        #[serde(default)]
        credentials: Option<yolo_hub_remote_setup_protocol::PrivateCredentials>,
    },
    RemoteSetup { call: yolo_hub_remote_setup_protocol::RemoteSetupCall },
    InspectRemoteSetup { #[serde(with = "crate::id")] operation_id: Uuid },
    List {
        include_archived: bool,
        after: Option<TerminalAddress>,
    },
    Get { address: TerminalAddress },
    Create {
        request: CreateShellRequest,
    },
    TerminalRun {
        request: TerminalRunRequest,
    },
    AgentStart {
        request: AgentStartRequest,
    },
    AgentStop { request: yolo_hub_agent_protocol::AgentStopRequest },
    CreateWorkspace {
        request: CreateWorkspaceRequest,
    },
    UpdateWorkspace {
        change: UpdateWorkspaceRequest,
    },
    GetWorkspace {
        request: GetWorkspaceRequest,
    },
    ListWorkspaces {
        request: ListWorkspacesRequest,
    },
    CreateWorktree { request: CreateWorktreeRequest },
    RemoveWorktree {
        change: RemoveWorktreeRequest,
    },
    ListWorktrees {
        request: ListWorktreesRequest,
    },
    GetAgentRun {
        request: GetAgentRunRequest,
    },
    ListAgentRuns {
        request: ListAgentRunsRequest,
    },
    UpdateAgentRun {
        change: UpdateAgentRunRequest,
    },
    QueryCreation {
        request: CreationQueryRequest,
    },
    Finalize {
        #[serde(with = "crate::id")]
        creation_token: Uuid,
        creation: CreateShellResult,
    },
    Lease {
        request: LeaseRequest,
    },
    Renew {
        grant: LeaseGrant,
    },
    Release {
        grant: LeaseGrant,
    },
    Input {
        request: InputRequest,
    },
    Resize { request: ResizeRequest },
    Terminal {
        command: TerminalCommand,
    },
    Attach { request: AttachRequest },
    ReadTerminalText { request: ReadTerminalTextRequest },
    ReadTerminalHistory { request: ReadTerminalHistoryRequest },
    ObserveLease {
        target: RuntimeTarget,
    },
    ListAgents {
        after: Option<AgentTarget>,
    },
    /// Long-poll on typed agent observations, evaluated by the Hub that owns
    /// the target; an entry Hub forwards a peer-addressed wait.
    WaitAgents {
        request: yolo_hub_agent_protocol::AgentWaitRequest,
    },
    GetAgent {
        target: AgentTarget,
    },
    GetCurrentAgent {
        address: AgentAddress,
    },
    ListHubs {
        after: Option<HubId>,
    },
    ListProviders {},
    /// Read signed audit JSONL lines after a sequence; a peer-addressed
    /// command lets an entry Hub pull another Hub's log.
    ExportAudit {
        after: Option<u64>,
        limit: u16,
    },
    CleanupTerminate {
        creation: CreateShellResult,
        force: bool,
    },
    CleanupArchive {
        creation: CreateShellResult,
    },
    GetConfiguration {},
    Configure {
        change: ConfigurationChange,
    },
    /// Mint a signed, expiring pairing code for a joining Hub.
    MintInvite {
        request: yolo_hub_enrollment_protocol::InviteRequest,
    },
    ListFileRoots {
        after: Option<FileRootId>,
    },
    ReadFile {
        request: ReadFileRequest,
    },
    AgentControl {
        request: AgentControlRequest,
        /// Origin and writer capability must match this runtime.
        grant: LeaseGrant,
    },
    AgentPrompt { request: yolo_hub_agent_protocol::AgentPromptRequest },
    GetPlatformSnapshot { request: crate::GetPlatformSnapshotRequest },
    ListPlatformEvents { request: yolo_hub_events_protocol::ListPlatformEventsRequest },
    SubscribeEvents { request: yolo_hub_events_protocol::SubscribeEventsRequest },
    ListLayouts {},
    GetLayout {
        address: yolo_presentation_protocol::LayoutAddress,
    },
    UpdateLayout {
        change: yolo_presentation_protocol::LayoutUpdate,
    },
    ResolveConversation {
        binding: yolo_conversation_protocol::ConversationBinding,
    },
    ReadConversation {
        request: yolo_conversation_protocol::ConversationReadRequest,
    },
    WatchConversation {
        request: yolo_conversation_protocol::ConversationReadRequest,
        wait_ms: u32,
    },
}
