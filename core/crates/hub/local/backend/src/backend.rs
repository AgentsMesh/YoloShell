use crate::errors::map_error;
use std::path::Path;
use yolo_hub_identity::HubId;
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::{CreateShellResult, PublicError, SecretCapability};

#[derive(Clone)]
pub struct LocalBackend {
    pub(crate) hub: HubId,
    pub(crate) client: yolo_client::Client,
    pub(crate) host_connections: yolo_hub_host_connections::HostConnections,
    pub(crate) paths: yolo_paths::Paths,
    pub(crate) file_helper: Option<std::path::PathBuf>,
    pub(crate) file_epoch: yolo_hub_identity::FileRootEpoch,
    pub(crate) files: Option<std::sync::Arc<yolo_hub_file_service::FileService>>,
    pub(crate) agents:
        Option<std::sync::Arc<tokio::sync::Mutex<yolo_hub_agent_projection::AgentProjection>>>,
    pub(crate) catalogue: std::sync::Arc<yolo_hub_host_catalogue::CatalogueCache>,
    pub(crate) connectors:
        Option<std::sync::Arc<tokio::sync::Mutex<yolo_hub_connector_registry::Registry>>>,
    pub(crate) connector_observation_gate: std::sync::Arc<tokio::sync::Mutex<()>>,
    pub(crate) observation_gate: std::sync::Arc<tokio::sync::Mutex<()>>,
    pub(crate) layouts:
        Option<std::sync::Arc<tokio::sync::Mutex<yolo_presentation_store::LayoutStore>>>,
    pub(crate) conversations: std::sync::Arc<yolo_conversation_service::ConversationReader>,
    pub(crate) launch: Option<yolo_agent_provider_contract::LaunchPolicies>,
    pub(crate) platform: Option<std::sync::Arc<yolo_hub_local_platform::PlatformServices>>,
    pub(crate) audit: Option<std::path::PathBuf>,
}

impl LocalBackend {
    pub fn new(hub: HubId, home: &Path, socket: &Path) -> Result<Self, PublicError> {
        let client = yolo_client::Client::managed_local(home, socket).map_err(map_error)?;
        Ok(Self {
            hub,
            client,
            host_connections: yolo_hub_host_connections::HostConnections::default(),
            paths: yolo_paths::Paths::new(home),
            file_helper: None,
            file_epoch: yolo_hub_identity::FileRootEpoch::new(),
            files: None,
            agents: None,
            catalogue: std::sync::Arc::new(yolo_hub_host_catalogue::CatalogueCache::new(
                yolo_hub_host_catalogue::CATALOGUE_TTL,
            )),
            connectors: None,
            connector_observation_gate: std::sync::Arc::new(tokio::sync::Mutex::new(())),
            observation_gate: std::sync::Arc::new(tokio::sync::Mutex::new(())),
            layouts: None,
            conversations: std::sync::Arc::new(yolo_conversation_service::ConversationReader::new(
                yolo_conversation_service::ConversationSources::new([]),
            )),
            launch: None,
            platform: None,
            audit: None,
        })
    }

    pub fn hub_id(&self) -> HubId {
        self.hub
    }

    pub fn with_audit(mut self, path: std::path::PathBuf) -> Self {
        self.audit = Some(path); self
    }

    pub async fn execute(&self, command: &Command) -> Result<Reply, PublicError> {
        command.validate_origin(self.hub)?;
        let reply = match self.platform_dispatch(command).await {
            Some(reply) => reply,
            None => self.route(command).await,
        };
        if command.affects_catalogue() { self.catalogue.invalidate().await; }
        if let (Some(platform), Ok(reply)) = (&self.platform, &reply) {
            platform.record(command, reply);
        }
        reply
    }

    async fn route(&self, command: &Command) -> Result<Reply, PublicError> {
        match command {
            Command::ListLayouts {} => self.list_layouts().await,
            Command::GetLayout { address } => self.get_layout(*address).await,
            Command::UpdateLayout { change } => self.update_layout(change.clone()).await,
            Command::ExportAudit { after, limit } => self.export_audit(*after, *limit),
            Command::ResolveConversation { binding } => self.resolve_conversation(*binding).await,
            Command::ReadConversation { request } => self.read_conversation(request).await,
            Command::WatchConversation { request, wait_ms } => {
                self.watch_conversation(request, *wait_ms).await
            }
            Command::GetClientAccess {}
            | Command::ConfigureClientAccess { .. }
            | Command::GetBuild {}
            | Command::GetConfiguration {}
            | Command::Configure { .. }
            | Command::MintInvite { .. }
            | Command::GetSshProfiles {}
            | Command::ConfigureSshProfiles { .. }
            | Command::ConnectSshProfile { .. }
            | Command::RemoteSetup { .. }
            | Command::WaitAgents { .. }
            | Command::InspectRemoteSetup { .. }
            | Command::CreateWorkspace { .. }
            | Command::UpdateWorkspace { .. }
            | Command::GetWorkspace { .. }
            | Command::ListWorkspaces { .. }
            | Command::GetAgentRun { .. }
            | Command::ListAgentRuns { .. }
            | Command::UpdateAgentRun { .. }
            | Command::AgentStop { .. }
            | Command::AgentPrompt { .. }
            | Command::ListPlatformEvents { .. }
            | Command::SubscribeEvents { .. }
            | Command::ListProviders {} | Command::CreateWorktree { .. }
            | Command::RemoveWorktree { .. }
            | Command::ListWorktrees { .. } => Err(PublicError::Unsupported),
            Command::Hello {} => {
                yolo_hub_local_platform::hello(&self.client, self.hub, |method| {
                    self.supports(method)
                })
                .await
            }
            Command::List {
                include_archived,
                after,
            } => self.list_page(*include_archived, *after).await,
            Command::Get { address } => Ok(Reply::Terminal {
                terminal: self.session(*address).await?,
            }),
            Command::Create { request } => {
                let created = self
                    .client
                    .create_session(&yolo_hub_local_platform::runtime_request(request))
                    .await
                    .map_err(map_error)?;
                let terminal_id = yolo_hub_identity::TerminalId::from_uuid(created.terminal_id)
                    .map_err(|_| PublicError::Internal)?;
                Ok(Reply::Created {
                    creation: CreateShellResult {
                        address: yolo_hub_identity::TerminalAddress {
                            hub_id: self.hub,
                            terminal_id,
                        },
                        creation_capability: SecretCapability::from_uuid(
                            created.creation_capability,
                        )?,
                    },
                })
            }
            Command::QueryCreation { request } => self.query_creation(request).await,
            Command::TerminalRun { request } => {
                let workspaces = self.platform.as_ref().map(|platform| platform.workspaces());
                yolo_hub_local_platform::terminal_run(self.hub, &self.client, workspaces, request)
                    .await
            }
            Command::AgentStart { request } => self.agent_start(request).await,
            Command::Finalize {
                creation_token,
                creation,
            } => {
                self.client
                    .finalize_creation(
                        *creation_token,
                        creation.address.terminal_id.as_uuid(),
                        creation.creation_capability.as_uuid(),
                    )
                    .await
                    .map_err(map_error)?;
                Ok(Reply::Completed {})
            }
            Command::Lease { request } => self.acquire(*request).await,
            Command::ObserveLease { target } => Ok(Reply::LeaseObservation {
                lease: self.observe_writer(*target).await?,
            }),
            Command::Renew { grant } => self.renew(*grant).await,
            Command::Release { grant } => self.release(*grant).await,
            Command::Input { request } => self.input(request).await,
            Command::Resize { request } => self.resize(*request).await,
            Command::Terminal { command } => self.lifecycle(*command).await,
            Command::Attach { .. } => Err(PublicError::Unsupported),
            Command::ReadTerminalText { request } => yolo_hub_local_platform::read_terminal_text(&self.client, request).await,
            Command::ReadTerminalHistory { .. } => Err(PublicError::Unsupported),
            Command::ListAgents { after } => self.list_agents(*after).await,
            Command::GetAgent { target } => self.get_agent(*target).await,
            Command::GetCurrentAgent { address } => self.get_current_agent(*address).await,
            Command::AgentControl { request, grant } => self.control_agent(request, *grant).await,
            Command::GetPlatformSnapshot { request } => self.platform_snapshot(request).await,
            Command::ListHubs { after } => Ok(yolo_hub_local_platform::list_hubs(self.hub, *after)),
            Command::CleanupTerminate { creation, force } => {
                self.cleanup_terminate(*creation, *force).await
            }
            Command::CleanupArchive { creation } => self.cleanup_archive(*creation).await,
            Command::ListFileRoots { after } => self.list_file_roots(*after),
            Command::ReadFile { request } => self.read_file(request.clone()).await,
        }
    }
}
