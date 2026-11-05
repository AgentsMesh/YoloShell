use crate::constants::*;
use anyhow::{Result, ensure};
use clap::Args;
use std::path::PathBuf;
use uuid::Uuid;
use yolo_hub_client::HubEndpoint;
use yolo_hub_identity::HubId;

#[derive(Debug, Default, Args)]
pub struct ConnectionOptions {
    /// Existing local Hub entry socket; supply together with --hub.
    #[arg(long, global = true, help_heading = "Connection", conflicts_with = ENDPOINT_ID)]
    pub socket: Option<PathBuf>,
    /// Hub entry endpoint (local:<socket>); supply together with --hub.
    #[arg(long, global = true, help_heading = "Connection", conflicts_with = SOCKET_ID)]
    pub endpoint: Option<HubEndpoint>,
    /// Identity of the Hub accepting this client connection.
    #[arg(long, global = true, help_heading = "Connection", conflicts_with = LOCAL_PROFILE_ID)]
    pub hub: Option<HubId>,
    /// Local profile file; defaults to the current user profile.
    #[arg(long, global = true, help_heading = "Connection", conflicts_with_all = [HUB_ID, SOCKET_ID, ENDPOINT_ID])]
    pub local_profile: Option<PathBuf>,
    /// Remote profile file; requires --account-key and --client-key.
    #[arg(long, global = true, help_heading = "Connection", requires_all = [ACCOUNT_KEY_ID, CLIENT_KEY_ID], conflicts_with_all = [HUB_ID, SOCKET_ID, ENDPOINT_ID, LOCAL_PROFILE_ID])]
    pub remote_profile: Option<PathBuf>,
    /// Account key paired with --remote-profile.
    #[arg(long, global = true, help_heading = "Connection", requires = REMOTE_PROFILE_ID)]
    pub account_key: Option<PathBuf>,
    /// Client key paired with --remote-profile.
    #[arg(long, global = true, help_heading = "Connection", requires = REMOTE_PROFILE_ID)]
    pub client_key: Option<PathBuf>,
    /// Attach a shared layout on the selected Hub; keyboard focus stays private.
    #[arg(long, global = true, help_heading = "Layout")]
    pub layout: Option<Uuid>,
}

impl ConnectionOptions {
    pub(crate) fn validate(&self) -> Result<()> {
        ensure!(
            self.hub.is_some() || (self.socket.is_none() && self.endpoint.is_none()),
            "{}",
            HUB_REQUIRES_CONNECTION
        );
        ensure!(
            self.hub.is_none() || self.socket.is_some() || self.endpoint.is_some(),
            "{}",
            CONNECTION_REQUIRES_HUB
        );
        Ok(())
    }

    pub(crate) fn has_any(&self) -> bool {
        self.socket.is_some()
            || self.endpoint.is_some()
            || self.hub.is_some()
            || self.local_profile.is_some()
            || self.remote_profile.is_some()
            || self.account_key.is_some()
            || self.client_key.is_some()
            || self.layout.is_some()
    }

    pub(crate) fn local_only(&self) -> bool {
        self.endpoint.is_none()
            && self.remote_profile.is_none()
            && self.account_key.is_none()
            && self.client_key.is_none()
            && self.layout.is_none()
    }
}
