use crate::{constants::*, selection::Selection};
use clap::{ArgGroup, Parser};
use std::path::PathBuf;
use yolo_hub_client::{ClientError, HubEndpoint};
use yolo_hub_identity::HubId;
use yolo_tui_app::Options;

#[derive(Debug, Parser)]
#[command(name = PROGRAM_NAME, version)]
#[command(about = ABOUT, after_help = CONTROLS)]
#[command(group(connection_group()))]
pub(crate) struct Arguments {
    /// Existing local Hub entry socket; supply together with --hub.
    #[arg(long, global = true, help_heading = "Connection", conflicts_with = ENDPOINT_ARGUMENT)]
    socket: Option<PathBuf>,
    /// Hub entry endpoint (local:<socket>); supply together with --hub.
    #[arg(long, global = true, help_heading = "Connection", conflicts_with = SOCKET_ARGUMENT)]
    endpoint: Option<HubEndpoint>,
    /// Identity of the Hub accepting this client connection.
    #[arg(long, global = true, help_heading = "Connection", requires = CONNECTION_GROUP, conflicts_with = LOCAL_PROFILE_ARGUMENT)]
    hub: Option<HubId>,
    /// Local profile file; defaults to the current user profile.
    #[arg(long, global = true, help_heading = "Connection", conflicts_with_all = [HUB_ARGUMENT, SOCKET_ARGUMENT, ENDPOINT_ARGUMENT])]
    local_profile: Option<PathBuf>,
    /// Remote profile file; requires --account-key and --client-key.
    #[arg(long, global = true, help_heading = "Connection", requires_all = [ACCOUNT_KEY_ARGUMENT, CLIENT_KEY_ARGUMENT], conflicts_with_all = [HUB_ARGUMENT, SOCKET_ARGUMENT, ENDPOINT_ARGUMENT, LOCAL_PROFILE_ARGUMENT])]
    remote_profile: Option<PathBuf>,
    /// Account key paired with --remote-profile.
    #[arg(long, global = true, help_heading = "Connection", requires = REMOTE_PROFILE_ARGUMENT)]
    account_key: Option<PathBuf>,
    /// Client key paired with --remote-profile.
    #[arg(long, global = true, help_heading = "Connection", requires = REMOTE_PROFILE_ARGUMENT)]
    client_key: Option<PathBuf>,
    /// Attach a shared layout on the selected Hub; keyboard focus stays private.
    #[arg(long, global = true, help_heading = "Layout")]
    layout: Option<uuid::Uuid>,
    #[command(subcommand)]
    pub(crate) command: Option<crate::command::Command>,
}

pub(crate) fn connection_group() -> ArgGroup {
    ArgGroup::new(CONNECTION_GROUP)
        .args([SOCKET_ARGUMENT, ENDPOINT_ARGUMENT])
        .requires(HUB_ARGUMENT)
}

impl Arguments {
    pub(crate) fn selection(self) -> anyhow::Result<Selection> {
        anyhow::ensure!(self.layout.is_none_or(|id| !id.is_nil()), INVALID_LAYOUT);
        for path in [self.local_profile.as_ref()].into_iter().flatten() {
            yolo_paths::validation::validate_absolute(path)?;
            anyhow::ensure!(
                path.to_str()
                    .is_some_and(|text| !text.chars().any(char::is_control)),
                INVALID_PATH
            );
        }
        if let Some(profile) = self.remote_profile {
            let mut options = crate::remote::options(
                &profile,
                self.account_key
                    .as_deref()
                    .ok_or(ClientError::InvalidEndpoint)?,
                self.client_key
                    .as_deref()
                    .ok_or(ClientError::InvalidEndpoint)?,
            )?;
            options.layout = self.layout;
            return Ok(Selection::Explicit(options));
        }
        match (self.hub, self.endpoint, self.socket) {
            (None, None, None) => Ok(Selection::Local {
                profile: self.local_profile,
                layout: self.layout,
            }),
            (Some(hub), Some(endpoint), None) => {
                endpoint.validate()?;
                Ok(Selection::Explicit(Options {
                    endpoint,
                    hub,
                    layout: self.layout,
                }))
            }
            (Some(hub), None, Some(socket)) => {
                let endpoint = HubEndpoint::local(socket);
                endpoint.validate()?;
                Ok(Selection::Explicit(Options {
                    endpoint,
                    hub,
                    layout: self.layout,
                }))
            }
            _ => Err(ClientError::InvalidEndpoint.into()),
        }
    }
}
