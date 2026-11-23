use crate::arguments::{Arguments, Command as CliCommand, WorkspaceCommand};
use clap::Parser;
use uuid::Uuid;
use yolo_hub_agent_run_protocol::{
    AgentRunAddress, AgentRunState, AgentRunSummary, ListAgentRunsResult,
};
use yolo_hub_identity::{
    AgentRunId, HubId, TerminalAddress, TerminalId, WorkspaceAddress, WorkspaceId,
};
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::{TerminalSummary, TerminalSummaryFields};
use yolo_hub_workspace_protocol::ListWorkspacesResult;

use crate::test_constants as constants;
#[path = "paging_peer.rs"]
mod peer;

#[tokio::test]
async fn a_qualified_workspace_cursor_needs_no_catalogue_lookup() {
    let entry = HubId::new();
    let cursor = WorkspaceAddress {
        hub_id: HubId::new(),
        workspace_id: WorkspaceId::new(),
    };
    let (mut client, server) = peer::client(entry, move |command| {
        let Command::ListWorkspaces { request } = command else {
            panic!("expected list")
        };
        assert_eq!(request.fields().after, Some(cursor));
        Reply::Workspaces {
            list: ListWorkspacesResult {
                workspaces: Vec::new(),
                next: None,
            },
        }
    })
    .await;
    let encoded = serde_json::to_string(&cursor).unwrap();
    let args = Arguments::try_parse_from([
        constants::PROGRAM,
        constants::WORKSPACE,
        constants::LIST,
        constants::AFTER,
        &encoded,
    ])
    .unwrap();
    let CliCommand::Workspace(options) = args.command else {
        panic!("expected workspace")
    };
    let WorkspaceCommand::List(list) = options.command else {
        panic!("expected list")
    };
    crate::listing::list(&mut client, entry, list)
        .await
        .unwrap();
    drop(client);
    assert_eq!(server.await.unwrap(), 1);
}

#[tokio::test]
async fn members_on_the_ninth_page_are_included_after_unrelated_terminals_and_runs() {
    const LAST: u128 = 9;
    let hub = HubId::new();
    let workspace = WorkspaceAddress {
        hub_id: hub,
        workspace_id: WorkspaceId::new(),
    };
    let (mut client, server) = peer::client(hub, move |command| match command {
        Command::List {
            include_archived: true,
            after,
        } => {
            let page = after.map_or(1, |cursor| cursor.terminal_id.as_uuid().as_u128() + 1);
            assert!(page <= LAST);
            let address = TerminalAddress {
                hub_id: hub,
                terminal_id: TerminalId::from_uuid(Uuid::from_u128(page)).unwrap(),
            };
            let row = TerminalSummary::new(TerminalSummaryFields {
                address,
                name: "shell".into(),
                cwd: "/tmp".into(),
                archived: true,
                runtime: None,
                agent: None,
                workspace: (page == LAST).then_some(workspace),
                presentation: Default::default(),
                recovery: None,
            })
            .unwrap();
            Reply::Terminals {
                terminals: vec![row],
                next: (page < LAST).then_some(address),
            }
        }
        Command::ListAgentRuns { request } => {
            let page = request
                .fields()
                .after
                .map_or(1, |cursor| cursor.run_id.as_uuid().as_u128() + 1);
            assert!(page <= LAST);
            let address = AgentRunAddress {
                hub_id: hub,
                run_id: AgentRunId::from_uuid(Uuid::from_u128(page)).unwrap(),
            };
            let row = AgentRunSummary {
                address,
                revision: 1,
                name: "run".into(),
                aliases: Vec::new(),
                labels: Default::default(),
                provider_id: "codex".into(),
                workspace: (page == LAST).then_some(workspace),
                parent: None,
                terminal: None,
                state: AgentRunState::Stopped,
                current: None,
                start_token: Uuid::new_v4(),
                created_at_ms: 0,
                updated_at_ms: 0,
            };
            Reply::AgentRuns {
                list: ListAgentRunsResult {
                    runs: vec![row],
                    next: (page < LAST).then_some(address),
                },
            }
        }
        _ => panic!("unexpected membership request"),
    })
    .await;
    let terminals = super::terminals(&mut client, hub, workspace).await.unwrap();
    let runs = super::runs(&mut client, hub, workspace).await.unwrap();
    assert_eq!(terminals.len(), 1);
    assert_eq!(terminals[0].fields().workspace, Some(workspace));
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].workspace, Some(workspace));
    drop(client);
    assert_eq!(server.await.unwrap(), LAST as usize * 2);
}

#[test]
fn repeated_or_backwards_cursors_fail_before_the_next_request() {
    let mut after = Some(2);
    assert!(super::advance(&mut after, Some(2)).is_err());
    assert!(super::advance(&mut after, Some(1)).is_err());
}
