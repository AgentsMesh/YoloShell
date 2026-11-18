use anyhow::{anyhow, Context, Result};
use yolo_trusted_plugin_manifest::{load, TrustedPluginManifest};
use yolo_trusted_plugin_registry::{Registry, RegistryPaths, RegistryRecord};

use crate::execution;

/// Stops a linked daemon before its source is replaced. Returns whether it
/// declared a stop action and was stopped.
pub(crate) fn stop_previous(paths: &RegistryPaths, previous: &RegistryRecord) -> Result<bool> {
    let manifest = match load(&previous.source) {
        Ok(manifest) => manifest,
        Err(_) => {
            eprintln!(
                "{}: cannot read the previous plugin source; restart it manually after the update",
                previous.id
            );
            return Ok(false);
        }
    };
    if !has_action(&manifest, "stop") {
        eprintln!(
            "{}: no stop action; restart it manually after the update",
            previous.id
        );
        return Ok(false);
    }
    execution::stop(paths, &previous.id)
        .with_context(|| format!("stop {} before the update", previous.id))?;
    Ok(true)
}

/// Starts the newly linked version when the previous one was stopped.
pub(crate) fn restart(paths: &RegistryPaths, id: &str, stopped: bool) -> Result<()> {
    if !stopped {
        return Ok(());
    }
    let registry = Registry::load(paths)?;
    let record = registry
        .get(id)
        .ok_or_else(|| anyhow!("plugin {id} is not linked"))?;
    let manifest = load(&record.source)?;
    if manifest.startup.is_none() {
        return Ok(());
    }
    execution::start_one(paths, id).with_context(|| {
        format!("installed {id} but could not restart it; run yolo plugin startup")
    })?;
    Ok(())
}

fn has_action(manifest: &TrustedPluginManifest, id: &str) -> bool {
    manifest.actions.iter().any(|action| action.id == id)
}
