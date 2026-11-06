use crate::fixtures::{constants::*, *};
use yolo_hub_rpc_protocol::{
    Command, HUB_PROTOCOL_VERSION, Hello, HubSummary, MAX_AUDIT_EXPORT_LIMIT, ProviderSummary,
    Reply, Request, SUPPORTED_METHODS,
};
#[path = "agent_runs.rs"]
mod agent_runs_support;
#[path = "agent_wait.rs"]
mod agent_wait_support;
#[path = "creation.rs"]
mod creation_support;
#[path = "files.rs"]
mod files;
#[path = "platform_events_fixtures.rs"]
mod platform_events_support;
#[path = "prompt_fixtures.rs"]
mod prompt_support;
#[path = "resources.rs"]
pub mod resources;
#[path = "snapshot_fixtures.rs"]
mod snapshot_support;
#[path = "text_fixtures.rs"]
mod text_support;
#[path = "workspaces.rs"]
mod workspaces_support;
pub use files::{file_read, file_reply, file_root};
pub fn samples() -> Vec<(Command, Reply)> {
    vec![
        (
            Command::AgentControl {
                request: agent_control(),
                grant: grant(),
            },
            Reply::AgentControl {
                receipt: agent_receipt(),
            },
        ),
        (
            Command::ListFileRoots { after: None },
            Reply::FileRoots {
                epoch: file_root().fields().target.epoch,
                roots: vec![file_root()],
                next: None,
            },
        ),
        (
            Command::ReadFile {
                request: file_read(),
            },
            Reply::File { file: file_reply() },
        ),
        (
            Command::CleanupTerminate {
                creation: creation(),
                force: false,
            },
            Reply::CreationTerminated {
                address: creation().address,
                outcome: yolo_hub_terminal_protocol::CleanupTerminationOutcome::Terminated,
            },
        ),
        (
            Command::CleanupArchive {
                creation: creation(),
            },
            Reply::Terminal {
                terminal: archived_summary(HUB_ID, SESSION_ID),
            },
        ),
        (
            Command::ListHubs { after: None },
            Reply::Hubs {
                hubs: vec![HubSummary {
                    hub_id: hub(HUB_ID),
                    reachable: true,
                    label: None,
                }],
                next: None,
            },
        ),
        (Command::ListProviders {}, Reply::Providers { providers: vec![ProviderSummary::new("codex", "Codex", true).unwrap()] }),
        (
            Command::ExportAudit {
                after: None,
                limit: MAX_AUDIT_EXPORT_LIMIT,
            },
            Reply::Audit { lines: Vec::new() },
        ),
        (
            Command::ListAgents { after: None },
            Reply::Agents {
                agents: vec![agent()],
                next: None,
            },
        ),
        (
            Command::WaitAgents {
                request: agent_wait_support::request(),
            },
            Reply::AgentWait {
                wait: agent_wait_support::result(),
            },
        ),
        (
            Command::GetAgent {
                target: agent().cursor().target(),
            },
            Reply::Agent { agent: agent() },
        ),
        (
            Command::GetCurrentAgent {
                address: agent().cursor().target().address,
            },
            Reply::Agent { agent: agent() },
        ),
        (
            Command::Hello {},
            Reply::Hello {
                hello: Hello {
                    hub_id: hub(HUB_ID),
                    version: HUB_PROTOCOL_VERSION,
                    methods: SUPPORTED_METHODS.to_vec(),
                },
            },
        ),
        (
            Command::List {
                include_archived: false,
                after: None,
            },
            Reply::Terminals {
                terminals: vec![summary(HUB_ID, SESSION_ID)],
                next: None,
            },
        ),
        (
            Command::Get {
                address: target().address,
            },
            Reply::Terminal {
                terminal: summary(HUB_ID, SESSION_ID),
            },
        ),
        (
            Command::Lease { request: lease() },
            Reply::Lease { grant: grant() },
        ),
        (
            Command::Renew { grant: grant() },
            Reply::Lease { grant: grant() },
        ),
        (Command::Release { grant: grant() }, Reply::Completed {}),
        (
            Command::Input { request: input() },
            Reply::Input { duplicate: false },
        ),
        (Command::Resize { request: resize() }, Reply::Completed {}),
        (
            Command::Terminal {
                command: session_command(),
            },
            Reply::Terminal {
                terminal: summary(HUB_ID, SESSION_ID),
            },
        ),
        (
            Command::Attach { request: attach() },
            Reply::Attached {
                attachment: attached(),
            },
        ),
        (
            Command::ObserveLease { target: target() },
            Reply::LeaseObservation {
                lease: Some(observation()),
            },
        ),
    ]
    .into_iter()
    .chain(resources::samples())
    .chain(creation_support::samples())
    .chain(workspaces_support::samples())
    .chain(agent_runs_support::samples())
    .chain(snapshot_support::samples())
    .chain(prompt_support::samples())
    .chain(platform_events_support::samples())
    .chain(text_support::samples())
    .collect()
}

pub fn request(command: Command) -> Request {
    Request::new(hub(HUB_ID), command).unwrap()
}

pub fn archived_summary(hub: u128, session: u128) -> yolo_hub_terminal_protocol::TerminalSummary {
    let mut fields = summary(hub, session).fields().clone();
    fields.archived = true;
    yolo_hub_terminal_protocol::TerminalSummary::new(fields).unwrap()
}
