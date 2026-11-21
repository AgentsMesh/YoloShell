mod configuration;
mod constants;
mod directory;
mod filesystem;
mod home;
mod installation;
mod model;
mod publication;
mod unix;

#[cfg(test)]
#[path = "../test_support/filesystem.rs"]
mod filesystem_test_support;

pub use configuration::{callback_command, contains_managed, merge};
pub use home::resolve_home;
pub use installation::{contains_managed_path, inspect, install, remove_legacy_backups};
pub use model::{HookProfile, Installation, IntegrationStatus, Preview};
pub use yolo_hook_installation_contract::{HookEventSpec, HookInstallSpec};
