pub(crate) const SOCKET_ID: &str = "socket";
pub(crate) const ENDPOINT_ID: &str = "endpoint";
pub(crate) const HUB_ID: &str = "hub";
pub(crate) const LOCAL_PROFILE_ID: &str = "local_profile";
pub(crate) const REMOTE_PROFILE_ID: &str = "remote_profile";
pub(crate) const ACCOUNT_KEY_ID: &str = "account_key";
pub(crate) const CLIENT_KEY_ID: &str = "client_key";

pub(crate) const LOCAL_CONNECTION_ONLY: &str =
    "`{command}` supports only --socket, --hub and --local-profile";
pub(crate) const NO_CONNECTION: &str = "`{command}` does not use Hub connection options";
pub const HUB_REQUIRES_CONNECTION: &str = "--socket and --endpoint require --hub";
pub const CONNECTION_REQUIRES_HUB: &str = "--hub requires --socket or --endpoint";
