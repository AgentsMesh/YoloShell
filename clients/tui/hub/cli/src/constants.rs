pub const INVALID_PUBLIC_KEY: &str = "Public key must be 64 hexadecimal characters";
pub const PEER_ALREADY_CONFIGURED: &str = "Peer is already configured";
pub const PEER_IS_LOCAL: &str = "The local Hub cannot be its own peer";
pub const INVALID_PEER_DIAL: &str = "Dial host and port must be supplied together";
pub const HUB_CONFIGURATION_READ_FAILED: &str = "Could not read the Hub configuration";
pub const HUB_CONFIGURATION_WRITE_FAILED: &str = "Could not change the Hub configuration";
pub const HUB_PEER_HEADER: &str = "HUB\tDIAL\tIMPORT\tEXPORTS";
pub const PEER_LIST_EMPTY: &str = "No peers configured. Mint an invite with `yolo hub peer invite` or join a Hub with `yolo hub peer join`.";
pub const PEER_LIST_LEGEND: &str =
    "IMPORT: this Hub imports the peer's resources; EXPORTS: origins this Hub exports to the peer";
pub const DEFAULT_PEER_PORT: u16 = 7400;
pub const DEFAULT_INVITE_MINUTES: u64 = 10;
pub const MIN_INVITE_MINUTES: u64 = 1;
pub const MAX_INVITE_MINUTES: u64 = 60;
pub const MINUTE_MS: u64 = 60 * 1000;
pub const INVITE_MINT_FAILED: &str = "Could not mint a pairing code";
pub const INVITE_LISTENER_NOT_APPLIED: &str =
    "The peer listener is not active; run `yolo hub listen <ip:port>` to retry it";
pub const JOIN_HOST_REQUIRED: &str =
    "Pass --host with the address of the inviting Hub; a yolo2_ pairing code does not contain one";
pub const INVITE_CODE_HEADER: &str = "Pairing code";
pub const INVITE_EXPIRES: &str = "Expires in";
pub const INVITE_JOIN_HINT: &str = "On the joining Hub, run: yolo hub peer join <code> --host <this Hub's address>";
pub const INVALID_INVITE_CODE: &str = "This is not a valid or unexpired pairing code";
pub const INVITE_POLICY_EMPTY: &str =
    "Pass --import-peer when --no-export is set; the pairing grants no direction";
pub const INVITE_TTL_INVALID: &str = "Pairing code lifetime must be between 1 and 60 minutes";
pub const ENROLLMENT_FAILED: &str = "The other Hub refused the pairing request";
pub const ENROLLMENT_UNAVAILABLE: &str = "Mint a fresh code and retry. If it persists, check that the same Hub is running, its listener is active and both sides use the same version";
pub const JOIN_CONNECT_FAILED: &str =
    "Could not connect to the inviting Hub; check --host, --port and that its listener is active";
pub const JOIN_HOST_KEY_MISMATCH: &str =
    "The address is not the inviting Hub (SSH host key mismatch); check --host and --port";
pub const JOIN_AUTHENTICATION_FAILED: &str =
    "The inviting Hub rejected the pairing code; mint a fresh code";
pub const JOIN_EXCHANGE_FAILED: &str =
    "The enrollment exchange failed; retry with a fresh code";
pub const JOINED: &str = "Joined";
pub const JOIN_PERSISTED_NOT_APPLIED: &str =
    "The other Hub persisted the peer but has not activated it yet";
pub const EXPIRES_AT_MS_FIELD: &str = "expires_at_ms";
pub const PUBLIC_KEY_FIELD: &str = "public_key";
pub const CONNECTION_GROUP: &str = "connection";
pub const SOCKET_ARGUMENT: &str = "socket";
pub const ENDPOINT_ARGUMENT: &str = "endpoint";
pub const HUB_ARGUMENT: &str = "hub";
