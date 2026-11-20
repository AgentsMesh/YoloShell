use crate::{Command, Reply};

pub fn reply_matches_target(reply: &Reply, command: &Command) -> bool {
    if let Some(valid) = crate::resource_reply::matches_target(reply, command) {
        return valid;
    }
    if let Some(valid) = crate::agent_run_reply::matches_target(reply, command) {
        return valid;
    }
    if let Some(valid) = crate::workspace_reply::matches_target(reply, command) {
        return valid;
    }
    if let Some(valid) = crate::event_reply::matches_target(reply, command) {
        return valid;
    }
    match (reply, command) {
        (Reply::Error { .. }, _) => true,
        (Reply::Hello { .. }, Command::Hello {}) => true,
        (Reply::Providers { .. }, Command::ListProviders {}) => true,
        (Reply::PlatformSnapshot { .. }, Command::GetPlatformSnapshot { .. }) => true,
        (Reply::Build { .. }, Command::GetBuild {}) => true,
        (Reply::Created { .. }, Command::Create { .. }) => true,
        (Reply::TerminalRun { run }, Command::TerminalRun { .. }) => {
            !run.address.terminal_id.as_uuid().is_nil()
        }
        (Reply::AgentStart { start }, Command::AgentStart { .. }) => {
            !start.address.terminal_id.as_uuid().is_nil()
                && start
                    .run
                    .as_ref()
                    .is_none_or(|run| run.terminal == Some(start.address))
        }
        (Reply::Completed {}, Command::Finalize { .. }) => true,
        (Reply::Completed {}, Command::Release { .. }) => true,
        (Reply::Completed {}, Command::Resize { .. }) => true,
        (Reply::Input { .. }, Command::Input { .. }) => true,
        (Reply::TerminalText { text }, Command::ReadTerminalText { request }) => {
            text.fields().target == request.fields().target
        }
        (Reply::TerminalHistory { history }, Command::ReadTerminalHistory { request }) => {
            history.fields().target == request.fields().target
        }
        (Reply::Layouts { .. }, Command::ListLayouts {}) => true,
        (Reply::SshProfiles { .. }, Command::GetSshProfiles {}) => true,
        (Reply::Configuration { .. }, Command::GetConfiguration {}) => true,
        (Reply::Consumers { .. }, Command::ListConsumers {}) => true,
        (Reply::ClientAccess { .. }, Command::GetClientAccess {}) => true,
        (
            Reply::ConversationUnavailable { .. },
            Command::ResolveConversation { .. }
            | Command::ReadConversation { .. }
            | Command::WatchConversation { .. },
        ) => true,
        (Reply::ClientAccess { configuration }, Command::ConfigureClientAccess { change }) => {
            configuration == &change.configuration
        }
        (Reply::CreationQuery { query }, Command::QueryCreation { request }) => {
            query.creation_token() == request.creation().fields().creation_token
        }
        (Reply::SshConnected { binding }, Command::ConnectSshProfile { profile, .. }) => {
            binding.profile.host_id == profile.profile_id()
                && binding.profile.profile_authority_sha256 == profile.authority_sha256()
        }
        (Reply::RemoteSetup { status }, Command::RemoteSetup { call }) => {
            status.validate_for(&call.request).is_ok()
        }
        (Reply::RemoteSetupHistory { history }, Command::InspectRemoteSetup { operation_id }) => {
            history.operation_id == *operation_id
        }
        (Reply::SshProfiles { state }, Command::ConfigureSshProfiles { change }) => {
            state.last_change == change.request_id
                && change
                    .expected_revision
                    .checked_add(yolo_hub_ssh_profile_protocol::SSH_PROFILE_REVISION_STEP)
                    == Some(state.revision)
                && state.configuration == change.configuration
        }
        (Reply::AgentControl { receipt }, Command::AgentControl { request, .. }) => {
            receipt.target() == request.fields().target
                && receipt.request_id() == request.fields().request_id
        }
        (Reply::AgentPrompt { prompt }, Command::AgentPrompt { request }) => {
            prompt.receipt.request_id() == request.fields().request_id
                && match &request.fields().reference {
                    yolo_hub_agent_protocol::AgentPromptReference::Execution { target } => {
                        prompt.receipt.target() == *target
                    }
                    yolo_hub_agent_protocol::AgentPromptReference::Terminal { address } => {
                        prompt.terminal == *address
                    }
                }
        }
        (Reply::File { file }, Command::ReadFile { request }) => file.validate_for(request).is_ok(),
        (Reply::FileRoots { roots, .. }, Command::ListFileRoots { after }) => {
            after.is_none_or(|after| {
                roots
                    .iter()
                    .all(|root| root.fields().target.address.root_id > after)
            })
        }
        (Reply::Configuration { snapshot }, Command::Configure { change }) => {
            snapshot.applied
                && snapshot.state.last_change == change.request_id
                && change
                    .expected_revision
                    .checked_add(yolo_hub_management_protocol::CONFIGURATION_REVISION_STEP)
                    == Some(snapshot.state.revision)
                && snapshot.state.configuration == change.configuration
        }
        (Reply::CreationTerminated { address, .. }, Command::CleanupTerminate { creation, .. }) => {
            *address == creation.address
        }
        (Reply::Terminal { terminal: session }, Command::CleanupArchive { creation }) => {
            session.fields().address == creation.address && session.fields().archived
        }
        (Reply::Hubs { hubs, .. }, Command::ListHubs { after }) => {
            after.is_none_or(|after| hubs.iter().all(|hub| hub.hub_id > after))
        }
        (Reply::Audit { lines }, Command::ExportAudit { limit, .. }) => {
            lines.len() <= usize::from(*limit)
        }
        (Reply::Agents { agents, .. }, Command::ListAgents { after }) => {
            after.is_none_or(|after| agents.iter().all(|agent| agent.cursor().target() > after))
        }
        (Reply::Agent { agent }, Command::GetAgent { target }) => {
            agent.cursor().target() == *target
        }
        (Reply::Agent { agent }, Command::GetCurrentAgent { address }) => {
            agent.cursor().target().address == *address
        }
        // The reply echoes every evaluated target so run selectors, which the
        // Hub expands, can be correlated without the client knowing them.
        (Reply::AgentWait { wait }, Command::WaitAgents { request }) => {
            let max = yolo_hub_agent_protocol::MAX_AGENT_WAIT_TARGETS;
            let evaluated = if request.runs.is_empty() {
                wait.targets.len() == request.targets.len()
            } else {
                true
            };
            wait.targets.len() <= max
                && wait.matched.len() <= max
                && wait.ended.len() <= max
                && evaluated
                && request
                    .targets
                    .iter()
                    .all(|target| wait.targets.contains(target))
                && wait
                    .ended
                    .iter()
                    .all(|target| wait.targets.contains(target))
                && wait
                    .matched
                    .iter()
                    .all(|agent| wait.targets.contains(&agent.cursor().target()))
        }
        (
            Reply::Terminals { terminals, .. },
            Command::List {
                after,
                include_archived,
            },
        ) => {
            (*include_archived || terminals.iter().all(|session| !session.fields().archived))
                && after.is_none_or(|after| {
                    terminals
                        .iter()
                        .all(|session| session.fields().address > after)
                })
        }
        (Reply::Terminal { terminal: session }, Command::Get { address }) => {
            session.fields().address == *address
        }
        (Reply::Terminal { terminal: session }, Command::Terminal { command }) => {
            session.fields().address == command.selection.address
        }
        (Reply::Lease { grant }, Command::Lease { request }) => {
            grant.fields().target == request.target && grant.fields().principal == request.principal
        }
        (Reply::Lease { grant }, Command::Renew { grant: previous }) => {
            let current = grant.fields();
            let previous = previous.fields();
            current.target == previous.target
                && current.principal == previous.principal
                && current.generation == previous.generation
                && current.capability == previous.capability
        }
        (Reply::LeaseObservation { lease }, Command::ObserveLease { target }) => lease
            .as_ref()
            .is_none_or(|lease| lease.fields().target == *target),
        (Reply::Attached { attachment }, Command::Attach { request }) => {
            attachment.validate_for_request(*request).is_ok()
        }
        // Every supported reply carries a correlation to the command that
        // produced it. A pair absent here is a protocol defect, not a reply
        // that is allowed to skip target validation.
        _ => false,
    }
}
