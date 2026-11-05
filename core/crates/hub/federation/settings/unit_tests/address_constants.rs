pub const SSH_TARGET: &str = "test-machine";
pub const SOCKET: &str = "/home/stone/.local/state/yolo/hub/peer.sock";
pub const SSH_FIELD: &str = "ssh";
pub const PROFILE_FIELD: &str = "profile";
pub const PROFILE_ID: &str = "test-profile";
pub const INVALID_SOCKETS: &[&str] = &[
    "",
    "peer.sock",
    "relative/peer.sock",
    "/tmp/../etc/peer.sock",
    "/tmp/./peer.sock",
    "/tmp/peer\n.sock",
    "/tmp/peer\0.sock",
];
