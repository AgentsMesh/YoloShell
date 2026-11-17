use std::time::Duration;

pub const ENTRY_PROFILE_SCHEMA: u16 = 1;
pub const MAX_PROFILE_BYTES: usize = 16_384;
pub(crate) const FIELD_SCHEMA_VERSION: &str = "schema_version";
pub(crate) const FIELD_HUB_ID: &str = "hub_id";
pub(crate) const FIELD_CLIENT_ID: &str = "client_id";
pub(crate) const MAX_HOST_BYTES: usize = 253;
pub(crate) const MAX_DNS_LABEL_BYTES: usize = 63;
pub(crate) const MAX_USERNAME_BYTES: usize = 64;
pub(crate) const MAX_HOST_KEY_BYTES: usize = 4_096;
pub(crate) const MAX_SOCKET_BYTES: usize = 103;
pub(crate) const SSH_CHANNEL_MESSAGES: usize = 32;
pub(crate) const SSH_WINDOW_BYTES: u32 = 65_536;
pub(crate) const SSH_PACKET_BYTES: u32 = 16_384;
pub(crate) const KEEPALIVE_INTERVAL: Duration = Duration::from_secs(10);
pub(crate) const MISSED_KEEPALIVES: usize = 3;
pub(crate) const CONNECT_FAILED: &str = "SSH entry connection failed";
pub(crate) const AUTH_FAILED: &str = "SSH entry account authentication failed";
pub(crate) const INGRESS_FAILED: &str = "SSH public entry channel was rejected";
pub(crate) const HUB_AUTH_FAILED: &str = "SSH public entry Hub authentication failed";
pub(crate) const CHANNEL_FAILED: &str = "SSH public entry channel is unavailable";
pub(crate) const ENTRY_CLOSED: &str = "SSH public entry is closed";
pub(crate) const EMPTY_KEY: [u8; yolo_hub_client_access_protocol::ED25519_PUBLIC_KEY_BYTES] =
    [0; yolo_hub_client_access_protocol::ED25519_PUBLIC_KEY_BYTES];
