use crate::{Hello, HubSummary};
use serde::{Deserialize, Serialize};
use yolo_hub_agent_protocol::{
    AgentPromptResult, AgentSnapshot, AgentStartResult, AgentStopResult, CommandReceipt,
};
use yolo_hub_agent_run_protocol::{AgentRunSummary, ListAgentRunsResult};
use yolo_hub_file_protocol::{FileRootSummary, ReadFileResult};
use yolo_hub_identity::{
    AgentTarget, FileRootEpoch, FileRootId, HubId, TerminalAddress, TerminalId,
};
use yolo_hub_terminal_protocol::{
    AttachResult, CleanupTerminationOutcome, CreateShellResult, CreationQueryResult, LeaseGrant,
    LeaseObservation, PublicError, TerminalHistoryRead, TerminalRunResult, TerminalSummary,
    TerminalText,
};
use yolo_hub_workspace_protocol::{ListWorkspacesResult, WorkspaceSummary};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "result", rename_all = "snake_case", deny_unknown_fields)]
pub enum Reply {
    Layouts {
        documents: Vec<yolo_presentation_protocol::LayoutSummary>,
    },
    Layout {
        document: Option<yolo_presentation_protocol::SharedLayoutDocument>,
    },
    Conversation {
        conversation: yolo_conversation_protocol::ConversationDescriptor,
    },
    ConversationRead {
        reading: yolo_conversation_protocol::ConversationReadResult,
    },
    ConversationUnavailable {
        error: yolo_conversation_protocol::ConversationError,
    },
    ClientAccess {
        configuration: yolo_hub_client_access_protocol::AccessConfiguration,
    },
    SshConnected {
        binding: yolo_broker_hub_protocol::BrokerHubConnectionBinding,
    },
    RemoteSetup {
        status: yolo_hub_remote_setup_protocol::RemoteSetupStatus,
    },
    RemoteSetupHistory {
        history: yolo_hub_remote_setup_protocol::RemoteSetupHistory,
    },
    SshProfiles {
        state: yolo_hub_ssh_profile_protocol::SshProfileState,
    },
    Build {
        build: yolo_hub_build_protocol::ServiceBuild,
    },
    AgentControl {
        receipt: CommandReceipt,
    },
    PlatformSnapshot {
        snapshot: crate::PlatformSnapshot,
    },
    Configuration {
        snapshot: yolo_hub_management_protocol::ConfigurationSnapshot,
    },
    Hubs {
        hubs: Vec<HubSummary>,
        next: Option<HubId>,
    },
    Providers {
        providers: Vec<crate::ProviderSummary>,
    },
    Audit {
        lines: Vec<serde_json::Value>,
    },
    Agents {
        agents: Vec<AgentSnapshot>,
        next: Option<AgentTarget>,
    },
    Agent {
        agent: AgentSnapshot,
    },
    AgentWait {
        wait: yolo_hub_agent_protocol::AgentWaitResult,
    },
    Hello {
        hello: Hello,
    },
    Terminals {
        terminals: Vec<TerminalSummary>,
        next: Option<TerminalAddress>,
    },
    Terminal {
        terminal: TerminalSummary,
    },
    Created {
        creation: CreateShellResult,
    },
    TerminalRun {
        run: TerminalRunResult,
    },
    AgentStart {
        start: AgentStartResult,
    },
    AgentStop {
        stop: AgentStopResult,
    },
    AgentPrompt {
        prompt: AgentPromptResult,
    },
    PlatformEvents {
        events: yolo_hub_events_protocol::ListPlatformEventsResult,
    },
    EventSubscription {
        subscription: yolo_hub_events_protocol::SubscribeEventsResult,
    },
    Workspace {
        workspace: WorkspaceSummary,
    },
    Workspaces {
        list: ListWorkspacesResult,
    },
    AgentRun {
        run: AgentRunSummary,
    },
    AgentRuns {
        list: ListAgentRunsResult,
    },
    CreationQuery {
        query: CreationQueryResult,
    },
    CreationTerminated {
        address: TerminalAddress,
        outcome: CleanupTerminationOutcome,
    },
    Lease {
        grant: LeaseGrant,
    },
    LeaseObservation {
        lease: Option<LeaseObservation>,
    },
    Input {
        duplicate: bool,
    },
    Attached {
        attachment: AttachResult,
    },
    TerminalText {
        text: TerminalText,
    },
    TerminalHistory {
        history: TerminalHistoryRead,
    },
    FileRoots {
        epoch: FileRootEpoch,
        roots: Vec<FileRootSummary>,
        next: Option<FileRootId>,
    },
    File {
        file: ReadFileResult,
    },
    Completed {},
    Error {
        code: PublicError,
    },
}

impl Reply {
    /// Fill run `current` fields from observed terminal-to-execution pairs.
    pub fn annotate_run_current(&mut self, observed: &[(TerminalId, AgentTarget)]) {
        match self {
            Reply::AgentRun { run } => {
                yolo_hub_agent_run_protocol::annotate_current(std::slice::from_mut(run), observed)
            }
            Reply::AgentRuns { list } => {
                yolo_hub_agent_run_protocol::annotate_current(&mut list.runs, observed)
            }
            _ => {}
        }
    }
}
