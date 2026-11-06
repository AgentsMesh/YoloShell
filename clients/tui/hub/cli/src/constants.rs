pub const INVALID_PUBLIC_KEY: &str = "Public key must be 64 hexadecimal characters";
pub const MACHINE_ALREADY_CONFIGURED: &str = "Machine is already configured";
pub const MACHINE_NOT_CONFIGURED: &str = "Machine is not configured";
pub const MACHINE_IS_LOCAL: &str = "The local machine cannot be added as a remote machine";
pub const MACHINE_IDENTITY_CHANGED: &str =
    "The configured machine identity changed; remove the machine before importing again";
pub const PEER_SOCKET_UNAVAILABLE: &str = "The local Hub did not report a peer socket";
pub const MACHINE_SSH_FAILED: &str = "Could not read the remote machine's Hub identity over SSH";
pub const MACHINE_OLDER_PROVIDER_HINT: &str =
    "the remote machine may run an older Yolo release; both machines need the same or a newer release";
pub const MACHINE_INVALID_RESPONSE: &str = "The remote machine's Hub identity response is invalid";
pub const REMOTE_YOLO_DEFAULT: &str = "yolo";
pub const INVALID_MACHINE_DIAL: &str = "Pass --ssh and --remote-socket together, or neither";
pub const HUB_CONFIGURATION_READ_FAILED: &str = "Could not read the Hub configuration";
pub const HUB_CONFIGURATION_WRITE_FAILED: &str = "Could not change the Hub configuration";
pub const MACHINE_LIST_HEADER: &str = "MACHINE\tHUB\tSOCKET\tIMPORTS FROM\tEXPORTS TO";
pub const MACHINE_LIST_EMPTY: &str =
    "No machines configured. Add one with `yolo machine add <user@host>`.";
pub const MACHINE_CONSUMERS_HEADER: &str = "HUB\tLINKS\tCONNECTED_MS";
pub const MACHINE_CONSUMERS_EMPTY: &str = "No machines are importing from this machine.";
pub const UNEXPECTED_REPLY: &str = "Hub returned an unexpected consumers reply";
pub const MACHINE_LIST_LEGEND: &str =
    "IMPORTS FROM: agents this machine imports from the remote machine; EXPORTS TO: origins this machine shares with it";
pub const IMPORT_ONE_WAY: &str = "one-way import; the remote machine does not import this machine";
pub const PUBLIC_KEY_FIELD: &str = "public_key";
pub const HUB_ID_FIELD: &str = "hub_id";
pub const SSH_PROGRAM: &str = "ssh";
pub const CONNECTION_GROUP: &str = "connection";
pub const SOCKET_ARGUMENT: &str = "socket";
pub const ENDPOINT_ARGUMENT: &str = "endpoint";
pub const HUB_ARGUMENT: &str = "hub";
