use crate::origin_services::OriginServices;
use crate::{constants, options::Options, services};
use anyhow::{Context, ensure};
use std::io::Write;
use std::sync::Arc;
use yolo_agent_providers::InteractionSemantics;
use yolo_hub_connector_service::ConnectorServer;
use yolo_hub_federation_settings::Configuration;
use yolo_hub_local_backend::LocalBackend;
use yolo_hub_management_runtime::ManagedHub;
use yolo_hub_rpc_protocol::{ProviderFeatures, ProviderInteraction, ProviderSummary};
use yolo_hub_server::HubServer;
use yolo_hub_state::HubState;

pub(crate) async fn run(options: Options) -> anyhow::Result<()> {
    let host_home = options.host_home.expect(constants::HOST_HOME_FIELD);
    let state_path = options.state.expect(constants::STATE_FIELD);
    let state = Arc::new(HubState::open(&state_path, &host_home)?);
    let hub = state.hub_id();
    let socket = state.socket_path();
    let host_socket = options
        .host_socket
        .map(Ok)
        .unwrap_or_else(|| yolo_paths::Paths::new(&host_home).host_socket())?;
    let program = match options.file_helper {
        Some(program) => program,
        None => std::env::current_exe()?,
    };
    let backend = LocalBackend::new(hub, &host_home, &host_socket)?;
    let directory = state.directory()?;
    let current = yolo_executable_identity::capture_current()
        .context("capture the running Hub executable image")?;
    let provider_runtime = yolo_agent_providers::builtin_provider_runtime();
    let providers = provider_runtime
        .runtimes()
        .map(|runtime| {
            let descriptor = runtime.descriptor();
            let capabilities = descriptor.capabilities();
            let features = ProviderFeatures {
                conversation: capabilities.conversation,
                submit_prompt: capabilities.submit,
                interrupt: capabilities.interrupt,
                interaction: match runtime.interaction_semantics() {
                    InteractionSemantics::Typed => ProviderInteraction::Typed,
                    InteractionSemantics::Terminal => ProviderInteraction::Terminal,
                    InteractionSemantics::None => ProviderInteraction::None,
                },
            };
            ProviderSummary::new(descriptor.kind().as_str(), descriptor.display_name(), false)
                .map(|summary| summary.with_features(features))
        })
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| anyhow::anyhow!("invalid built-in provider catalog"))?;
    let backend = backend
        .with_agents(
            directory.ensure_descendant(&directory.path().join(constants::AGENT_DIRECTORY))?,
            yolo_agent_providers::builtin_projection_policies(),
        )?
        .with_launch_policies(yolo_agent_providers::builtin_launch_policies())
        .with_platform(
            directory.ensure_descendant(&directory.path().join(constants::WORKSPACE_DIRECTORY))?,
            directory.ensure_descendant(&directory.path().join(constants::AGENT_RUN_DIRECTORY))?,
        )?
        .with_conversations(yolo_agent_providers::builtin_conversation_sources())
        .with_file_helper(program)?
        .with_audit(directory.path().join(yolo_hub_audit::AUDIT_FILE));
    let _host_hold = backend.retain_host().await?;
    let build = backend.build_provider()?;
    let observed = build
        .build()
        .await
        .map_err(|error| anyhow::anyhow!("observe Host and file-helper images: {error}"))?;
    ensure!(observed.all_match(&current), constants::MIXED_SERVICE_IMAGE);
    let backend = backend.with_connectors(
        directory.ensure_descendant(
            &directory
                .path()
                .join(yolo_hub_connector_store::CONNECTOR_DIRECTORY),
        )?,
        state.peer_identity(),
    )?;
    let connectors = ConnectorServer::bind(state.clone(), backend.clone()).await?;
    let config: Option<Configuration> = options
        .peer_config
        .as_ref()
        .map(|path| yolo_paths::read_json(path))
        .transpose()?;
    let public_identity = serde_json::to_string(&state.peer_identity())?;
    let broker_program = match options.broker_executable {
        Some(program) => program,
        None => std::env::current_exe()?,
    };
    let profiles = yolo_hub_ssh_profile_runtime::SshProfileRuntime::open_with_launcher(
        state.clone(),
        options
            .ssh_profiles
            .as_ref()
            .map(|path| yolo_paths::read_json(path))
            .transpose()?,
        Some(yolo_hub_ssh_profile_runtime::BrokerExecutable {
            executable: broker_program,
            host_home: host_home.clone(),
        }),
    )?;
    let host_client = yolo_client::Client::managed_local(&host_home, &host_socket)
        .context("open the Host provider-status connection")?;
    let origin = Arc::new(OriginServices {
        hub,
        host: host_client.clone(),
        providers: providers.clone(),
    });
    let (managed, service) = ManagedHub::open_with_origin(
        state.clone(),
        backend,
        config,
        Some(build),
        Arc::new(yolo_hub_peer_dialer::CompositeDialer::new(
            yolo_hub_peer_dialer::SystemSshDialer::default(),
            profiles.clone(),
        )),
        origin,
    )
    .await?;
    let remote = yolo_hub_remote_setup_runtime::RemoteSetupRuntime::open(
        state.clone(),
        profiles.clone(),
        managed,
    )?;
    let history = Arc::new(yolo_hub_local_terminal_history::HistoryDispatcher::new(
        remote,
        hub,
        host_client.clone(),
    ));
    let provider_inner = Arc::new(yolo_hub_local_providers::ProviderDispatcher::new(
        history,
        hub,
        host_client,
        providers,
    ));
    let access = yolo_hub_client_access_runtime::ClientAccessRuntime::open(
        state.clone(),
        profiles.dispatcher(provider_inner)?,
        options
            .client_access
            .as_ref()
            .map(|path| yolo_paths::read_json(path))
            .transpose()?,
    )?;
    let audit_path = state.directory()?.path().join(yolo_hub_audit::AUDIT_FILE);
    let audit = Arc::new(
        yolo_hub_audit::AuditLog::open(audit_path, state.clone())
            .context(constants::AUDIT_OPEN_FAILED)?,
    );
    let public = yolo_hub_public_client_server::PublicClientServer::bind_audited(
        state.clone(),
        access.clone(),
        access.registry(),
        Some(audit.clone()),
    )
    .await?;
    let server = HubServer::bind_dispatcher_audited(state, access.clone(), Some(audit)).await?;
    let terminate = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
    if options.ready_stdio {
        let document = yolo_hub_local_startup::ReadyDocument {
            schema_version: yolo_hub_local_startup::CONFIG_SCHEMA_VERSION,
            entry_hub: hub,
            socket_path: socket,
        };
        let mut output = std::io::stdout().lock();
        output.write_all(&yolo_hub_local_startup::encode_ready(&document)?)?;
        output.flush()?;
    } else {
        println!("Yolo Hub {hub}\n{}\n{public_identity}", socket.display());
        println!("{}", connectors.socket_path().display());
        println!("{}", public.socket_path().display());
        if let Ok(peer_socket) = service.peer_socket() {
            println!("{}", peer_socket.display());
        }
    }
    services::run(server, connectors, service, public, access, terminate).await
}
