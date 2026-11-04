//! Reply/method semantic validation. The envelope checks version, origin and
//! correlation; this module checks that a decoded reply is well formed for the
//! method it answers.

use crate::{
    MAX_CATALOG_PAGE_AGENTS, MAX_CATALOG_PAGE_HUBS, MAX_CATALOG_PAGE_TERMINALS, Method, OPERATIONS,
    Reply,
};
use yolo_hub_identity::HubId;

pub(crate) fn valid(reply: &Reply, method: Method, origin: HubId, version: u16) -> bool {
    crate::resource_reply::valid(reply, method, origin)
        .or_else(|| crate::agent_reply::valid(reply, method, origin))
        .unwrap_or_else(|| match (reply, method) {
            (
                Reply::ClientAccess { configuration },
                Method::GetClientAccess | Method::ConfigureClientAccess,
            ) => configuration.entry_hub == origin && configuration.validate().is_ok(),
            (Reply::SshConnected { binding }, Method::ConnectSshProfile) => {
                binding.validate().is_ok()
            }
            (Reply::RemoteSetup { .. }, Method::RemoteSetup) => true,
            (Reply::RemoteSetupHistory { history }, Method::InspectRemoteSetup) => {
                history.validate_for(origin, history.operation_id).is_ok()
            }
            (
                Reply::SshProfiles { state },
                Method::GetSshProfiles | Method::ConfigureSshProfiles,
            ) => state.validate().is_ok() && state.hub.hub() == origin,
            (Reply::Build { .. }, Method::GetBuild) => true,
            (Reply::Audit { lines }, Method::ExportAudit) => {
                lines.len() <= usize::from(crate::MAX_AUDIT_EXPORT_LIMIT)
            }
            (Reply::PlatformSnapshot { snapshot }, Method::GetPlatformSnapshot) => {
                let fields = snapshot.fields();
                fields
                    .terminals
                    .iter()
                    .all(|terminal| terminal.fields().address.hub_id == origin)
                    && fields
                        .agents
                        .iter()
                        .all(|agent| agent.cursor().target().address.hub_id == origin)
                    && fields.runs.iter().all(|run| run.address.hub_id == origin)
                    && fields
                        .workspaces
                        .iter()
                        .all(|workspace| workspace.address.hub_id == origin)
                    && fields.hubs.iter().any(|hub| hub.hub_id == origin)
            }
            (Reply::PlatformEvents { events }, Method::ListPlatformEvents) => {
                events
                    .events
                    .iter()
                    .all(|event| event.kind.hub_id() == Some(origin))
                    && events
                        .events
                        .windows(2)
                        .all(|pair| pair[0].sequence < pair[1].sequence)
                    && events
                        .events
                        .iter()
                        .all(|event| event.sequence <= events.stream.sequence)
            }
            (Reply::EventSubscription { subscription }, Method::SubscribeEvents) => {
                (!subscription.timed_out || subscription.events.is_empty())
                    && subscription
                        .events
                        .iter()
                        .all(|event| event.kind.hub_id() == Some(origin))
                    && subscription
                        .events
                        .windows(2)
                        .all(|pair| pair[0].sequence < pair[1].sequence)
                    && subscription
                        .events
                        .iter()
                        .all(|event| event.sequence <= subscription.stream.sequence)
            }
            (Reply::TerminalRun { run }, Method::TerminalRun) => run.address.hub_id == origin,
            (Reply::Workspace { workspace }, _) => workspace.address.hub_id == origin,
            (Reply::Workspaces { list }, Method::ListWorktrees) => list
                .workspaces
                .iter()
                .all(|workspace| workspace.address.hub_id == origin),
            (Reply::Workspaces { list }, Method::ListWorkspaces) => list
                .workspaces
                .windows(2)
                .all(|pair| pair[0].address < pair[1].address),
            (Reply::AgentRun { run }, _) => run.address.hub_id == origin,
            (Reply::AgentRuns { list }, Method::ListAgentRuns) => list
                .runs
                .windows(2)
                .all(|pair| pair[0].address < pair[1].address),
            (Reply::FileRoots { epoch, roots, next }, Method::ListFileRoots) => {
                roots.len() <= yolo_hub_file_protocol::MAX_FILE_ROOT_PAGE
                    && roots.iter().all(|root| {
                        root.fields().target.address.hub_id == origin
                            && root.fields().target.epoch == *epoch
                    })
                    && roots.windows(2).all(|pair| {
                        pair[0].fields().target.address.root_id
                            < pair[1].fields().target.address.root_id
                    })
                    && next.is_none_or(|next| {
                        roots
                            .last()
                            .is_some_and(|root| root.fields().target.address.root_id == next)
                    })
            }
            (Reply::File { file }, Method::ReadFile) => {
                file.fields().target.address.hub_id == origin
            }
            (Reply::Configuration { snapshot }, Method::GetConfiguration | Method::Configure) => {
                snapshot.validate().is_ok() && snapshot.state.identity.hub() == origin
            }
            (Reply::Minted { invite }, Method::MintInvite) => invite.validate().is_ok(),
            (Reply::Error { .. }, _) => true,
            (Reply::Hubs { hubs, next }, Method::ListHubs) => {
                hubs.len() <= MAX_CATALOG_PAGE_HUBS
                    && hubs.windows(2).all(|pair| pair[0].hub_id < pair[1].hub_id)
                    && next.is_none_or(|next| hubs.last().is_some_and(|hub| hub.hub_id == next))
            }
            (Reply::Agents { agents, next }, Method::ListAgents) => {
                agents.len() <= MAX_CATALOG_PAGE_AGENTS
                    && agents
                        .windows(2)
                        .all(|pair| pair[0].cursor().target() < pair[1].cursor().target())
                    && next.is_none_or(|next| {
                        agents
                            .last()
                            .is_some_and(|agent| agent.cursor().target() == next)
                    })
            }
            (Reply::Providers { providers }, Method::ListProviders) => {
                providers.len() <= crate::MAX_PROVIDERS
                    && providers
                        .windows(2)
                        .all(|pair| pair[0].provider_id < pair[1].provider_id)
                    && providers.iter().all(|provider| provider.validate().is_ok())
            }
            (Reply::Hello { hello }, Method::Hello) => {
                hello.hub_id == origin
                    && hello.version == version
                    && hello.methods.len() <= OPERATIONS.len()
                    && hello
                        .methods
                        .iter()
                        .enumerate()
                        .all(|(i, method)| !hello.methods[..i].contains(method))
            }
            (Reply::Terminals { terminals, next }, Method::List) => {
                terminals.len() <= MAX_CATALOG_PAGE_TERMINALS
                    && terminals
                        .windows(2)
                        .all(|pair| pair[0].fields().address < pair[1].fields().address)
                    && next.is_none_or(|next| {
                        terminals
                            .last()
                            .is_some_and(|session| session.fields().address == next)
                    })
            }
            (
                Reply::Terminal { terminal: session },
                Method::Get | Method::Terminal | Method::CleanupArchive,
            ) => session.fields().address.hub_id == origin,
            (Reply::CreationTerminated { address, .. }, Method::CleanupTerminate) => {
                address.hub_id == origin
            }
            (Reply::Created { creation }, Method::Create) => creation.address.hub_id == origin,
            (Reply::CreationQuery { query }, Method::QueryCreation) => {
                query.validate_for(query.creation_token(), origin).is_ok()
            }
            (Reply::Lease { grant }, Method::Lease | Method::Renew) => {
                grant.fields().target.address.hub_id == origin
            }
            (Reply::LeaseObservation { lease }, Method::ObserveLease) => lease
                .as_ref()
                .is_none_or(|lease| lease.fields().target.address.hub_id == origin),
            (Reply::Input { .. }, Method::Input) => true,
            (Reply::TerminalText { text }, Method::ReadTerminalText) => {
                text.fields().target.address.hub_id == origin
            }
            (Reply::TerminalHistory { history }, Method::ReadTerminalHistory) => {
                history.fields().target.address.hub_id == origin
            }
            (Reply::Attached { attachment }, Method::Attach) => {
                attachment.fields().target.address.hub_id == origin
            }
            (
                Reply::Completed {},
                Method::Finalize | Method::Release | Method::Resize | Method::RemoveWorktree,
            ) => true,
            _ => false,
        })
}
