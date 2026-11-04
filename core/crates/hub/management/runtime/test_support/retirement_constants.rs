use std::time::Duration;

pub const TEST_TIMEOUT: Duration = Duration::from_secs(15);
pub const COMPLETED_BEFORE_RELEASE: &str = "configuration completed before retired work released";
pub const RETIRED_BEFORE_LOCK_RELEASE: &str = "retirement completed with a generation reader held";
pub const NETWORK_COMPLETION_FAILURE: &str = "injected old network completion failure";
