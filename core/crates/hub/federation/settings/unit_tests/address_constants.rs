pub const PROFILE: &str = "test-machine";
pub const DIGEST_CHARACTER: &str = "a";
pub const DIGEST_BYTES: usize = 64;
pub const PORT: u16 = 2022;
pub const TRANSPORT: &str = "transport";
pub const KIND: &str = "kind";
pub const PROFILE_FIELD: &str = "profile";
pub const INVALID_HOSTS: &[&str] = &["", "bad host", "bad\nhost"];
pub const INVALID_SSH_HOSTS: &[&str] = &[
    "localhost",
    "192.0.2.1",
    "::1%lo0",
    "127.00.0.1",
    "127.0.0.1\n",
];
pub const LOOPBACK: &str = "127.0.0.1";
pub const V6_LOOPBACK: &str = "::1";
pub const UNKNOWN_KIND: &str = "unrecognized";
