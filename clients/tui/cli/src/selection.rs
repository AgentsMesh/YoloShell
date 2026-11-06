use crate::command::Command;
use crate::constants::{LIFECYCLE_LOCAL_ONLY, MANAGEMENT_LOCAL_ONLY, MANAGEMENT_TIMEOUT};
use crate::recipe::RecipeCommand;
use std::path::{Path, PathBuf};
use yolo_hub_client::HubEndpoint;
use yolo_hub_state::HubState;
use yolo_local_profile::LocalProfile;
use yolo_tui_app::Options;

pub(crate) enum Selection {
    Explicit(Options),
    Local {
        profile: Option<PathBuf>,
        layout: Option<uuid::Uuid>,
    },
}

impl Selection {
    pub async fn run(self, command: Option<Command>) -> anyhow::Result<()> {
        if let Some(
            command @ (Command::Status(_)
            | Command::Version(_)
            | Command::Start(_)
            | Command::Stop(_)
            | Command::Restart(_)),
        ) = &command
        {
            let profile = self.local_profile()?;
            return match command {
                Command::Status(options) => {
                    yolo_tui_service::status_command(&profile, *options).await
                }
                Command::Version(options) => {
                    yolo_tui_service::version_command(&profile, *options).await
                }
                Command::Start(options) => {
                    yolo_tui_service::start_command(&profile, *options).await
                }
                Command::Stop(options) => yolo_tui_service::stop_command(&profile, *options).await,
                Command::Restart(options) => {
                    yolo_tui_service::restart_command(&profile, *options).await
                }
                _ => unreachable!(),
            };
        }
        if let Some(Command::Machine { command }) = &command {
            let (socket, entry) = self.local_hub_endpoint().await?;
            let deadline = tokio::time::Instant::now() + MANAGEMENT_TIMEOUT;
            return yolo_tui_hub_cli::run(&socket, entry, command, deadline).await;
        }
        if let Some(Command::Audit {
            after,
            limit,
            merge,
            trust,
            identity,
            pull,
            output,
            json,
        }) = &command
        {
            return crate::audit::run(
                &self,
                *after,
                *limit,
                *json,
                merge,
                trust,
                *identity,
                *pull,
                output.clone(),
            )
            .await;
        }
        if let Some(Command::Recipe {
            command: RecipeCommand::Show(args),
        }) = &command
        {
            return crate::recipe::show(args);
        }
        if matches!(
            command,
            Some(Command::Detach { .. } | Command::Clients { .. })
        ) {
            return crate::detach::run(self, command.expect("matched command")).await;
        }
        match self {
            Self::Explicit(options) => run(options, std::future::pending(), command).await,
            Self::Local { profile, layout } => {
                if command.is_some() {
                    notice_if_starting(profile.as_deref())?;
                }
                let mut startup = yolo_tui_local_startup::automatic(profile.as_deref()).await?;
                let endpoint = HubEndpoint::local(startup.connection.socket_path.clone());
                endpoint.validate()?;
                let options = Options {
                    endpoint,
                    hub: startup.connection.entry_hub,
                    layout,
                };
                run(options, startup.cancelled(), command).await
            }
        }
    }

    async fn local_hub_endpoint(&self) -> anyhow::Result<(PathBuf, yolo_hub_identity::HubId)> {
        match self {
            Self::Explicit(options) => {
                let Some(socket) = options.endpoint.local_socket() else {
                    anyhow::bail!("{}", MANAGEMENT_LOCAL_ONLY);
                };
                Ok((socket.to_path_buf(), options.hub))
            }
            Self::Local { profile, .. } => {
                notice_if_starting(profile.as_deref())?;
                let startup = yolo_tui_local_startup::automatic(profile.as_deref()).await?;
                Ok((
                    startup.connection.socket_path.clone(),
                    startup.connection.entry_hub,
                ))
            }
        }
    }

    fn local_profile(&self) -> anyhow::Result<LocalProfile> {
        match self {
            Self::Local { profile, .. } => match profile {
                Some(path) => LocalProfile::load(path),
                None => yolo_local_profile::current(),
            },
            Self::Explicit(_) => anyhow::bail!("{}", LIFECYCLE_LOCAL_ONLY),
        }
    }
}

fn notice_if_starting(profile_path: Option<&Path>) -> anyhow::Result<()> {
    let profile = match profile_path {
        Some(path) => LocalProfile::load(path)?,
        None => yolo_local_profile::current()?,
    };
    let running = HubState::socket_path_for(&profile.state).is_ok_and(|path| path.exists());
    if !running {
        eprintln!("{}", crate::constants::STARTING_LOCAL_SERVICES);
    }
    Ok(())
}

async fn run(
    options: Options,
    shutdown: impl std::future::Future<Output = ()>,
    command: Option<Command>,
) -> anyhow::Result<()> {
    match command {
        Some(Command::List { json, all }) => crate::listing::run(&options, json, all).await,
        Some(Command::Layouts { json }) => crate::layouts::run(&options, json).await,
        Some(Command::Platform { command }) => crate::platform::run(&options, command).await,
        Some(Command::Screen { terminal, json }) => {
            crate::screen::run(&options, terminal, json).await
        }
        Some(Command::Recipe {
            command: RecipeCommand::Start(args),
        }) => crate::recipe::start(&options, &args).await,
        Some(Command::Recipe {
            command: RecipeCommand::Show(_),
        }) => unreachable!("recipe show is dispatched before connection setup"),
        Some(command @ (Command::Archive { .. } | Command::Terminate { .. })) => {
            crate::lifecycle::run(&options, command).await
        }
        command => {
            let action =
                crate::command::startup(command, options.endpoint.local_socket().is_none())?;
            yolo_tui_app::run_with_startup(options, shutdown, action).await
        }
    }
}
