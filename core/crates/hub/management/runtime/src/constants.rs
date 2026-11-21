pub(crate) const MANAGEMENT_QUEUE_CAPACITY: usize = 16;
pub(crate) const GENERATION_FAILED: &str = "Hub federation generation stopped";
pub(crate) const GENERATION_RETAINED: &str = "retired Hub federation is still retained";
pub(crate) const UPDATE_FAILED: &str = "Hub configuration activation failed; federation disabled";
pub(crate) const ACTIVATION_RETRY_FAILED: &str =
    "durable Hub configuration is still awaiting federation activation";
pub(crate) const ACTIVATION_RECOVERED: &str =
    "durable Hub configuration activated after internal retry";
pub(crate) const ACTIVATION_RETRY_INTERVAL: std::time::Duration = std::time::Duration::from_secs(5);
pub(crate) const CONFIGURATION_VERIFY_INTERVAL: std::time::Duration =
    std::time::Duration::from_secs(1);
pub(crate) const CONFIGURATION_LOST: &str = "Hub configuration custody was lost";
pub(crate) const NO_DIALER: &str = "this Hub has no SSH peer dialer";
