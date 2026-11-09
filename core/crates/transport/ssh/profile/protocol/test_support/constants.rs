pub const PROFILE: &str = "remote-machine_1";
pub const REVISION_CHARACTER: &str = "a";
pub const NUMERIC_REVISION_CHARACTER: &str = "7";
pub const PROFILE_FIELD: &str = "profile_id";
pub const REVISION_FIELD: &str = "authority_sha256";
pub const UNKNOWN_FIELD: &str = "private_key";
pub const INVALID_PROFILES: &[&str] = &["", ".", "..", "a/b", "a b", "a\n", "机器"];
