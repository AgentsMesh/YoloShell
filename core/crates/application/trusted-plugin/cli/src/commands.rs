use std::path::PathBuf;

use anyhow::Result;
use yolo_trusted_plugin_manifest::load;
use yolo_trusted_plugin_registry::{Registry, RegistryPaths, source_digest};

use crate::arguments::{Arguments, Command as RootCommand, PluginCommand};

pub(crate) fn run(arguments: Arguments) -> Result<()> {
    let RootCommand::Plugin(options) = arguments.command;
    let paths = crate::paths::registry_paths()?;
    match options.command {
        PluginCommand::List { json } => list(&paths, json),
        PluginCommand::Describe { id, json } => describe(&paths, &id, json),
        PluginCommand::Link { dir, trust, json } => link(&paths, &dir, trust, json),
        PluginCommand::Install {
            id,
            version,
            registry,
            trust,
            json,
        } => crate::install::install(
            &paths,
            &id,
            version.as_deref(),
            registry.as_deref(),
            trust,
            json,
        ),
        PluginCommand::Unlink { id } => mutate(&paths, &id, None),
        PluginCommand::Enable { id } => mutate(&paths, &id, Some(true)),
        PluginCommand::Disable { id } => mutate(&paths, &id, Some(false)),
        PluginCommand::Startup { json } => crate::execution::startup(&paths, json),
        PluginCommand::Action { id, action, json } => {
            crate::execution::invoke_action(&paths, &id, &action, json)
        }
        PluginCommand::Status { id, json } => status(&paths, &id, json),
        PluginCommand::Log { id } => log(&paths, &id),
        PluginCommand::DataDir { id } => print_path(&paths, &id),
    }
}

fn list(paths: &RegistryPaths, json: bool) -> Result<()> {
    let registry = Registry::load(paths)?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&registry.records().collect::<Vec<_>>())?
        );
    } else {
        for record in registry.records() {
            println!(
                "{} {} {}",
                record.id,
                if record.enabled {
                    "enabled"
                } else {
                    "disabled"
                },
                record.source.display()
            );
        }
    }
    Ok(())
}

fn describe(paths: &RegistryPaths, id: &str, json: bool) -> Result<()> {
    let registry = Registry::load(paths)?;
    let (record, manifest) = registry.revalidate(id)?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(
                &serde_json::json!({"record": record, "manifest": manifest})
            )?
        );
    } else {
        println!(
            "id: {}\nname: {}\nversion: {}\nsource: {}\ndigest: {}\nplatforms: {:?}\nweb: {:?}\napi: {:?}",
            manifest.id,
            manifest.name,
            manifest.version,
            record.source.display(),
            record.digest,
            record.platforms,
            manifest.web,
            manifest.api
        );
    }
    Ok(())
}

fn link(paths: &RegistryPaths, dir: &PathBuf, trust: bool, json: bool) -> Result<()> {
    let manifest = load(dir)?;
    let digest = source_digest(dir)?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(
                &serde_json::json!({"manifest": manifest, "digest": digest})
            )?
        );
    } else {
        println!(
            "id: {}\nstartup: {:?}\nactions: {:?}\ndigest: {}",
            manifest.id, manifest.startup, manifest.actions, digest
        );
    }
    if !trust {
        anyhow::bail!("review the commands and digest above, then rerun with --trust");
    }
    let mut registry = Registry::load_locked(paths)?;
    registry.link_approved(dir, &digest, paths)?;
    Ok(())
}

fn mutate(paths: &RegistryPaths, id: &str, enabled: Option<bool>) -> Result<()> {
    let mut registry = Registry::load_locked(paths)?;
    if let Some(enabled) = enabled {
        registry.set_enabled(id, enabled, paths)?;
    } else {
        registry.unlink(id, paths)?;
    }
    Ok(())
}

fn status(paths: &RegistryPaths, id: &str, json: bool) -> Result<()> {
    let registry = Registry::load(paths)?;
    let record = registry
        .get(id)
        .ok_or_else(|| anyhow::anyhow!("plugin is not linked"))?;
    if json {
        println!("{}", serde_json::to_string_pretty(record)?);
    } else {
        println!("{}: {:?}", id, record.last_startup);
    }
    Ok(())
}

fn log(paths: &RegistryPaths, id: &str) -> Result<()> {
    let registry = Registry::load(paths)?;
    registry
        .get(id)
        .ok_or_else(|| anyhow::anyhow!("plugin is not linked"))?;
    let Ok(entries) = std::fs::read_dir(paths.log_dir(id)) else {
        println!("no captured log");
        return Ok(());
    };
    let mut newest: Option<(std::time::SystemTime, PathBuf)> = None;
    for entry in entries.flatten() {
        let Ok(metadata) = entry.metadata() else {
            continue;
        };
        if !metadata.is_file() {
            continue;
        }
        let Ok(modified) = metadata.modified() else {
            continue;
        };
        if newest.as_ref().map_or(true, |(time, _)| modified > *time) {
            newest = Some((modified, entry.path()));
        }
    }
    let Some((_, path)) = newest else {
        println!("no captured log");
        return Ok(());
    };
    let bytes = std::fs::read(path)?;
    let start = bytes.len().saturating_sub(64 * 1024);
    print!("{}", String::from_utf8_lossy(&bytes[start..]));
    Ok(())
}

fn print_path(paths: &RegistryPaths, id: &str) -> Result<()> {
    let registry = Registry::load(paths)?;
    registry
        .get(id)
        .ok_or_else(|| anyhow::anyhow!("plugin is not linked"))?;
    println!("{}", paths.data_dir(id).display());
    Ok(())
}
