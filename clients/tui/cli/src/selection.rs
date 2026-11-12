use crate::command::Command;
use crate::recipe::RecipeCommand;
use std::path::PathBuf;
use yolo_hub_client::HubEndpoint;
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
