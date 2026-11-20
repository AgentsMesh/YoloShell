use anyhow::{Context, Result};
use clap::{Args, Subcommand};
use std::path::PathBuf;
use yolo_hook_installation::{
    HookInstallSpec, HookProfile, IntegrationStatus, contains_managed_path, inspect,
    install as install_hooks, remove_legacy_backups, resolve_home,
};
use yolo_agent_providers::{builtin_install_spec, builtin_install_specs};

const PROVIDER_UNAVAILABLE: &str = "provider has no hook installation profile";

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
    let path = resolve_home(provider, explicit, |name| std::env::var_os(name))?;
    yolo_runtime::paths::validation::validate_absolute(&path)?;
    Ok(path)
}
