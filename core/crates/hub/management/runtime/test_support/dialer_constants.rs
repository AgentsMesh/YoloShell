use std::time::Duration;
pub const DIALER_TEST_TIMEOUT: Duration = Duration::from_secs(15);
pub const FIRST_PROFILE: &str = "configured-first";
pub const SECOND_PROFILE: &str = "configured-second";
pub const PROFILE_REVISION: &str =
    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
pub const FIRST_SOCKET: &str = "/run/user/1000/yolo/first.sock";
pub const SECOND_SOCKET: &str = "/run/user/1000/yolo/second.sock";
pub const HOST_DIRECTORY: &str = "host";
pub const HUB_DIRECTORY: &str = "hub";
pub const REMOTE_DIRECTORY: &str = "remote";
pub const REMOTE_HOME_DIRECTORY: &str = "remote-home";
pub const ABSENT_SOCKET: &str = "absent-host.sock";
pub const DIALER_BUFFER_BYTES: usize = 1024;
pub const FIRST_DIAL: usize = 1;
pub const SECOND_DIAL: usize = 2;
pub const DIAL_STEP: usize = 1;
