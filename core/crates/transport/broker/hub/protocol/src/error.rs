use thiserror::Error;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Error)]
pub enum BrokerHubError {
    #[error("invalid Hub broker profile identity")]
    Profile,
    #[error("invalid Hub broker connection binding")]
    Binding,
    #[error("Hub endpoint must be a nonzero unscoped loopback address")]
    Endpoint,
    #[error("broker is not configured for Hub transport")]
    Mode,
    #[error("Hub transport configuration cannot contain a Host runtime artifact")]
    Artifact,
    #[error("Hub broker profile or configuration differs from the expected identity")]
    ProfileMismatch,
    #[error("Hub broker connection binding is stale")]
    StaleBinding,
    #[error("Hub broker channel differs from the requested identity")]
    ChannelMismatch,
    #[error("Hub broker tunnel capacity is exhausted")]
    Capacity,
}

impl BrokerHubError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::Profile | Self::Binding | Self::Endpoint => "invalid_hub_request",
            Self::Mode | Self::Artifact => "hub_transport_disabled",
            Self::ProfileMismatch => "hub_profile_mismatch",
            Self::StaleBinding => "hub_connection_stale",
            Self::ChannelMismatch => "hub_channel_mismatch",
            Self::Capacity => "hub_tunnel_capacity",
        }
    }
}
