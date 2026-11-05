pub const INVALID_PUBLIC_KEY: &str = "Public key must be 64 hexadecimal characters";
pub const PEER_ALREADY_CONFIGURED: &str = "Peer is already configured";
pub const PEER_IS_LOCAL: &str = "The local Hub cannot be its own peer";
pub const INVALID_PEER_DIAL: &str =
    "Pass --ssh-profile and --remote-socket together, or neither";
pub const HUB_CONFIGURATION_READ_FAILED: &str = "Could not read the Hub configuration";
pub const HUB_CONFIGURATION_WRITE_FAILED: &str = "Could not change the Hub configuration";
pub const HUB_PEER_HEADER: &str = "HUB\tSSH PROFILE:SOCKET\tIMPORT\tEXPORTS";
pub const PEER_LIST_EMPTY: &str =
    "No peers configured. Add one with `yolo hub peer add <hub-id> <key> --ssh-profile <profile> --remote-socket <path>`.";
pub const PEER_LIST_LEGEND: &str =
    "IMPORT: this Hub imports the peer's resources; EXPORTS: origins this Hub exports to the peer";
pub const PUBLIC_KEY_FIELD: &str = "public_key";
pub const CONNECTION_GROUP: &str = "connection";
pub const SOCKET_ARGUMENT: &str = "socket";
pub const ENDPOINT_ARGUMENT: &str = "endpoint";
pub const HUB_ARGUMENT: &str = "hub";
