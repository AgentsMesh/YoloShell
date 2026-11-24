//! Provider-neutral hook installation profile data. A provider catalog supplies
//! these specs; the installation engine owns the filesystem/merge safety.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HookEventSpec {
    pub name: &'static str,
    pub timeout_seconds: Option<u64>,
}

impl HookEventSpec {
    pub const fn new(name: &'static str) -> Self {
        Self {
            name,
            timeout_seconds: None,
        }
    }

    pub const fn with_timeout(name: &'static str, seconds: u64) -> Self {
        Self {
            name,
            timeout_seconds: Some(seconds),
        }
    }
}

/// One provider's hook installation profile. `provider` is the stable CLI token
/// (for example `codex`), not an agent kind or manifest ID.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HookInstallSpec {
    pub provider: &'static str,
    pub display_name: &'static str,
    pub file_name: &'static str,
    /// Directory below the provider's base home that holds the config file
    /// (empty when the file sits directly in the base home).
    pub subdirectory: &'static str,
    pub environment_name: &'static str,
    pub directory_name: &'static str,
    /// Arguments appended after the Host role, for example
    /// `provider-hook --provider grok`.
    pub hook_arguments: &'static str,
    /// Command fragments that identify this provider's managed callbacks,
    /// including legacy forms that must be rewritten on upgrade.
    pub managed_hook_commands: &'static [&'static str],
    pub events: &'static [HookEventSpec],
    pub restricted_keys: &'static [&'static str],
    /// Nested settings paths whose explicit `false` disables hook execution.
    pub disabled_paths: &'static [&'static [&'static str]],
    pub requires_provider_trust: bool,
}
