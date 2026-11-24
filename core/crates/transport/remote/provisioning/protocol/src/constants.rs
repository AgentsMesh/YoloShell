pub const PROVISION_SCHEMA_VERSION: u16 = 1;
pub const PROVISION_OPERATION_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(300);
pub const PROVISION_CAPABILITY: &str = "hub_provisioning";
pub(crate) const INVALID: &str = "invalid Hub provisioning request or outcome";
