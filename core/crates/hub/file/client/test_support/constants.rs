pub const ROOT_NAME: &str = "Workspace";
pub const ROOT_PATH: &str = "/srv/project";
pub const RELATIVE_FILE: &str = "src/main.rs";
pub const PAYLOAD: &[u8] = b"hello";
pub const CHANNEL_CAPACITY: usize = 4096;
pub const REQUEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(3);
