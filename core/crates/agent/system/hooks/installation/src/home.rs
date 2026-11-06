use anyhow::{Context, Result};
use std::ffi::OsString;
use std::path::PathBuf;
use yolo_hook_installation_contract::HookInstallSpec;

const HOME_UNAVAILABLE: &str = "provider configuration home is unavailable";
const USER_HOME_ENV: &str = "HOME";

/// Resolve the directory that holds one provider's hook configuration.
///
/// Precedence: the explicit `--config-home`, the provider's data-home
/// environment variable, then `$HOME/<directory_name>`. The profile's
/// subdirectory is appended last so Grok keeps its `hooks/` directory.
pub fn resolve_home(
    spec: &HookInstallSpec,
    explicit: Option<PathBuf>,
    var_os: impl Fn(&str) -> Option<OsString>,
) -> Result<PathBuf> {
    let base = explicit
        .or_else(|| {
            var_os(spec.environment_name)
                .filter(|value| !value.is_empty())
                .map(PathBuf::from)
        })
        .or_else(|| var_os(USER_HOME_ENV).map(|home| PathBuf::from(home).join(spec.directory_name)))
        .context(HOME_UNAVAILABLE)?;
    Ok(if spec.subdirectory.is_empty() {
        base
    } else {
        base.join(spec.subdirectory)
    })
}

#[cfg(test)]
#[path = "../unit_tests/home.rs"]
mod tests;
