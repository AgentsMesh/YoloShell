use std::collections::BTreeSet;
use yolo_hub_routing::{Right, Rights};
use yolo_hub_rpc_protocol::*;

#[test]
fn operation_table_defines_every_method_once_and_keeps_the_wire_groups_explicit() {
    assert_eq!(ALL_METHODS.len(), OPERATIONS.len());
    let unique = ALL_METHODS.iter().copied().collect::<BTreeSet<_>>();
    assert_eq!(unique.len(), ALL_METHODS.len());
    for method in ALL_METHODS {
        assert_eq!(method.operation().method, *method);
    }

    // These literals are deliberately independent of the macro-generated
    // tables, so a method moved into the wrong advertised group fails here.
    assert_eq!(
        SUPPORTED_METHODS,
        &[
            Method::ListLayouts,
            Method::GetLayout,
            Method::UpdateLayout,
            Method::ResolveConversation,
            Method::ReadConversation,
            Method::WatchConversation,
            Method::Hello,
            Method::List,
            Method::Get,
            Method::Create,
            Method::TerminalRun,
            Method::AgentStart,
            Method::AgentStop,
            Method::CreateWorkspace,
            Method::UpdateWorkspace,
            Method::GetWorkspace,
            Method::ListWorkspaces,
            Method::CreateWorktree,
            Method::RemoveWorktree,
            Method::ListWorktrees,
            Method::GetAgentRun,
            Method::ListAgentRuns,
            Method::UpdateAgentRun,
            Method::QueryCreation,
            Method::Finalize,
            Method::Lease,
            Method::Renew,
            Method::Release,
            Method::Input,
            Method::Resize,
            Method::Terminal,
            Method::Attach,
            Method::ReadTerminalText,
            Method::ReadTerminalHistory,
            Method::ObserveLease,
            Method::ListAgents,
            Method::WaitAgents,
            Method::GetAgent,
            Method::GetCurrentAgent,
            Method::ListHubs,
            Method::ListProviders,
            Method::GetPlatformSnapshot,
            Method::ListPlatformEvents,
            Method::ExportAudit,
            Method::SubscribeEvents,
            Method::CleanupTerminate,
            Method::CleanupArchive,
            Method::ListFileRoots,
            Method::ReadFile,
            Method::AgentControl,
            Method::AgentPrompt,
        ]
    );
    assert_eq!(
        MANAGEMENT_METHODS,
        &[Method::GetConfiguration, Method::Configure, Method::MintInvite]
    );
    assert_eq!(BUILD_METHODS, &[Method::GetBuild]);
    assert_eq!(
        SSH_PROFILE_METHODS,
        &[
            Method::GetSshProfiles,
            Method::ConfigureSshProfiles,
            Method::ConnectSshProfile,
        ]
    );
    assert_eq!(
        REMOTE_SETUP_METHODS,
        &[Method::RemoteSetup, Method::InspectRemoteSetup]
    );
    assert_eq!(
        CLIENT_ACCESS_METHODS,
        &[Method::GetClientAccess, Method::ConfigureClientAccess]
    );
}

#[test]
fn operation_table_derives_access_mutation_and_local_management() {
    let agent_control: Rights = [Right::AgentControl, Right::TerminalWrite]
        .into_iter()
        .collect();
    assert_eq!(
        Method::AgentControl.required_rights(),
        Some(agent_control.clone())
    );
    assert_eq!(Method::AgentPrompt.required_rights(), Some(agent_control));
    assert!(Method::AgentPrompt.mutates());
    let terminal_run: Rights = [Right::Lifecycle, Right::TerminalWrite]
        .into_iter()
        .collect();
    assert_eq!(Method::TerminalRun.required_rights(), Some(terminal_run));
    assert!(Method::TerminalRun.mutates());
    let agent_start: Rights = [Right::AgentControl, Right::Lifecycle, Right::TerminalWrite]
        .into_iter()
        .collect();
    assert_eq!(Method::AgentStart.required_rights(), Some(agent_start));
    assert!(Method::AgentStart.mutates());
    let agent_lifecycle: Rights = [Right::AgentControl, Right::Lifecycle]
        .into_iter()
        .collect();
    assert_eq!(Method::AgentStop.required_rights(), Some(agent_lifecycle));
    assert!(Method::AgentStop.mutates());
    let workspace_read: Rights = [Right::WorkspaceRead].into_iter().collect();
    let workspace_write: Rights = [Right::WorkspaceWrite].into_iter().collect();
    assert_eq!(
        Method::GetWorkspace.required_rights(),
        Some(workspace_read.clone())
    );
    assert_eq!(
        Method::ListWorkspaces.required_rights(),
        Some(workspace_read)
    );
    assert_eq!(
        Method::CreateWorkspace.required_rights(),
        Some(workspace_write.clone())
    );
    assert_eq!(
        Method::UpdateWorkspace.required_rights(),
        Some(workspace_write)
    );
    assert!(!Method::ListWorkspaces.mutates());
    assert!(Method::CreateWorkspace.mutates());
    let agent_metadata: Rights = [Right::AgentControl].into_iter().collect();
    assert_eq!(
        Method::UpdateAgentRun.required_rights(),
        Some(agent_metadata)
    );
    assert_eq!(
        Method::GetAgentRun.required_rights(),
        Some([Right::Observe].into_iter().collect())
    );
    assert!(!Method::ListAgentRuns.mutates());
    assert!(Method::UpdateAgentRun.mutates());
    assert_eq!(Method::Configure.required_rights(), None);
    assert!(!Method::List.mutates());
    assert!(Method::Configure.mutates());
    assert!(Method::RemoteSetup.mutates());
    assert!(Method::ReadFile.operation().requires_read_reply_admission());
    assert!(
        Method::ListLayouts
            .operation()
            .requires_read_reply_admission()
    );
    assert!(!Method::List.operation().requires_read_reply_admission());
    assert!(Command::GetConfiguration {}.local_only());
    assert!(Method::MintInvite.operation().access == Access::LocalManagement);
    assert!(!Method::MintInvite.mutates());
    assert!(!Command::Hello {}.local_only());
}
