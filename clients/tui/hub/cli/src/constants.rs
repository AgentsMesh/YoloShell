pub const INVALID_PUBLIC_KEY: &str = "Public key must be 64 hexadecimal characters";
pub const PEER_ALREADY_CONFIGURED: &str = "Peer is already configured";
pub const PEER_IS_LOCAL: &str = "The local Hub cannot be its own peer";
pub const INVALID_PEER_DIAL: &str = "Dial host and port must be supplied together";
pub const HUB_CONFIGURATION_READ_FAILED: &str = "Could not read the Hub configuration";
pub const HUB_CONFIGURATION_WRITE_FAILED: &str = "Could not change the Hub configuration";
pub const HUB_PEER_HEADER: &str = "HUB\tDIAL\tIMPORT\tEXPORTS";
pub const DEFAULT_INVITE_MINUTES: u64 = 10;
pub const MIN_INVITE_MINUTES: u64 = 1;
pub const MAX_INVITE_MINUTES: u64 = 60;
pub const MINUTE_MS: u64 = 60 * 1000;
pub const INVITE_MINT_FAILED: &str = "Could not mint a pairing code";
pub const INVITE_DIAL_REQUIRED: &str =
    "Pass --host and --port: the Hub has no applied, dialable listener address";
pub const INVITE_CODE_HEADER: &str = "Pairing code";
pub const INVITE_EXPIRES: &str = "Expires at";
pub const INVITE_JOIN_HINT: &str = "On the joining Hub, run: yolo hub peer join <code>";
pub const INVALID_INVITE_CODE: &str = "This is not a valid or unexpired pairing code";
pub const INVITE_POLICY_EMPTY: &str =
    "Pass --import-peer when --no-export is set; the pairing grants no direction";
pub const INVITE_TTL_INVALID: &str = "Pairing code lifetime must be between 1 and 60 minutes";
pub const ENROLLMENT_FAILED: &str = "The other Hub refused the pairing request";
pub const JOINED: &str = "Joined";
pub const JOIN_PERSISTED_NOT_APPLIED: &str =
    "The other Hub persisted the peer but has not activated it yet";
pub const PUBLIC_KEY_FIELD: &str = "public_key";
pub const CONNECTION_GROUP: &str = "connection";
pub const SOCKET_ARGUMENT: &str = "socket";
pub const ENDPOINT_ARGUMENT: &str = "endpoint";
pub const HUB_ARGUMENT: &str = "hub";
