use anyhow::{Context, Result};
use clap::{Args, Subcommand};
use std::path::PathBuf;
use yolo_hook_installation::{
    HookInstallSpec, HookProfile, IntegrationStatus, contains_managed_path, inspect,
    install as install_hooks, remove_legacy_backups,
};
use yolo_agent_providers::{builtin_install_spec, builtin_install_specs};

const HOME_UNAVAILABLE: &str = "provider configuration home is unavailable";
const PROVIDER_UNAVAILABLE: &str = "provider has no hook installation profile";
const USER_HOME_ENV: &str = "HOME";

#[derive(Debug, Args)]
pub(super) struct Options {
    #[command(subcommand)]
    pub command: Operation,
}

#[derive(Debug, Args)]
pub(super) struct Target {
    #[arg(long)]
    pub provider: String,
    #[arg(long = yolo_runtime::cli::constants::CONFIG_HOME_NAME)]
    pub config_home: Option<PathBuf>,
}

#[derive(Debug, Subcommand)]
pub(super) enum Operation {
    ProviderProfiles,
    Inspect(Target),
    Install {
        #[command(flatten)]
        target: Target,
        #[arg(long)]
        expected_revision: String,
    },
}

pub(super) fn run(options: Options) -> Result<()> {
    let output = match options.command {
        Operation::ProviderProfiles => {
            let profiles: Vec<HookProfile> = builtin_install_specs()
                .into_iter()
                .map(HookProfile::from)
                .collect();
            yolo_capnp_codec::to_value(&profiles)?
        }
        Operation::Inspect(target) => {
            let provider = provider(&target.provider)?;
            yolo_capnp_codec::to_value(&inspect(provider, &home(provider, target.config_home)?)?)?
        }
        Operation::Install {
            target,
            expected_revision,
        } => {
            let provider = provider(&target.provider)?;
            yolo_capnp_codec::to_value(&install_hooks(
                provider,
                &home(provider, target.config_home)?,
                &expected_revision,
            )?)?
        }
    };
    use std::io::Write;
    let mut stdout = std::io::stdout().lock();
    stdout.write_all(&yolo_capnp_codec::encode_value(&output)?)?;
    stdout.flush()?;
    Ok(())
}

/// Rewrite stale managed callbacks left by earlier releases. Runs at Host
/// startup; a provider whose file has no managed callback is never installed.
pub(super) fn migrate_installed() {
    for spec in builtin_install_specs() {
        if let Err(error) = migrate(spec) {
            tracing::warn!(
                provider = spec.provider,
                %error,
                "provider hook migration skipped"
            );
        }
    }
}

fn migrate(spec: &HookInstallSpec) -> Result<()> {
    let Ok(home) = home(spec, None) else {
        return Ok(());
    };
    remove_legacy_backups(&home)?;
    if !contains_managed_path(spec, &home)? {
        return Ok(());
    }
    let preview = inspect(spec, &home)?;
    if preview.status != IntegrationStatus::Missing {
        return Ok(());
    }
    install_hooks(spec, &home, &preview.revision)?;
    Ok(())
}

fn provider(token: &str) -> Result<&'static HookInstallSpec> {
    builtin_install_spec(token).context(PROVIDER_UNAVAILABLE)
}

fn home(provider: &HookInstallSpec, explicit: Option<PathBuf>) -> Result<PathBuf> {
    let path = explicit
        .or_else(|| {
            std::env::var_os(provider.environment_name)
                .filter(|value| !value.is_empty())
                .map(PathBuf::from)
        })
        .or_else(|| {
            std::env::var_os(USER_HOME_ENV)
                .map(|home| PathBuf::from(home).join(provider.directory_name))
        })
        .context(HOME_UNAVAILABLE)?;
    let path = if provider.subdirectory.is_empty() {
        path
    } else {
        path.join(provider.subdirectory)
    };
    yolo_runtime::paths::validation::validate_absolute(&path)?;
    Ok(path)
}
