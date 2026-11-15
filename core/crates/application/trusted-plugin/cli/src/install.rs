use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};
use yolo_trusted_plugin_manifest::load;
use yolo_trusted_plugin_registry::{Registry, RegistryPaths, source_digest};

use crate::install_git;
use crate::install_lifecycle;
use crate::install_registry::{DEFAULT_REGISTRY, RegistrySpec, load_spec};

pub(crate) fn install(
    paths: &RegistryPaths,
    id: &str,
    version: Option<&str>,
    registry: Option<&str>,
    trust: bool,
    json: bool,
) -> Result<()> {
    let registry_url = registry.unwrap_or(DEFAULT_REGISTRY);
    let temporary = tempfile::tempdir().context("create plugin install directory")?;
    let registry_dir = temporary.path().join("registry");
    install_git::clone_registry(registry_url, &registry_dir)?;
    let spec = load_spec(&registry_dir, id, version)?;

    let source_dir = temporary.path().join("source");
    install_git::clone_source(&spec.source.repository, &spec.source.tag, &source_dir)?;
    let head = install_git::head(&source_dir)?;
    if head != spec.source.commit {
        bail!(
            "registry source commit mismatch: expected {}, got {}",
            spec.source.commit,
            head
        );
    }

    let prepared = temporary.path().join("prepared");
    install_git::copy_plugin(&source_dir.join(&spec.source.path), &prepared)?;
    let manifest = load(&prepared).context("load plugin manifest from registry source")?;
    if manifest != spec.manifest {
        bail!("registry manifest does not match the plugin source manifest");
    }
    let digest = source_digest(&prepared)?;
    preview(&spec, &digest, json)?;
    if !trust {
        bail!("review the source and digest above, then rerun with --trust");
    }

    let previous = Registry::load(paths)?.get(&spec.id).cloned();
    let target = installed_path(paths, &spec.id, &spec.version);
    let existing = target.exists();
    if existing {
        let current = source_digest(&target)?;
        if current != digest {
            bail!(
                "{} {} already exists at {} with a different digest; remove it to reinstall",
                spec.id,
                spec.version,
                target.display()
            );
        }
    }
    let stopped = match &previous {
        Some(previous) => install_lifecycle::stop_previous(paths, previous)?,
        None => false,
    };
    if !existing {
        if let Err(error) = commit_source(&target, &prepared, &digest) {
            restart_previous(paths, &spec.id, stopped);
            return Err(error);
        }
    }
    let linked = Registry::load_locked(paths).and_then(|mut registry| {
        registry.replace_approved(&target, &digest, paths)
    });
    if let Err(error) = linked {
        if !existing {
            let _ = std::fs::remove_dir_all(&target);
        }
        restart_previous(paths, &spec.id, stopped);
        return Err(error.into());
    }
    if previous.is_some() {
        install_lifecycle::restart(paths, &spec.id, stopped)?;
    }
    if !json {
        if previous.is_some() {
            println!("updated {} to {} at {}", spec.id, spec.version, target.display());
        } else {
            println!("installed {} {} at {}", spec.id, spec.version, target.display());
        }
    }
    Ok(())
}

fn commit_source(target: &Path, prepared: &Path, digest: &str) -> Result<()> {
    install_git::copy_plugin(prepared, target)?;
    let copied_digest = source_digest(target)?;
    if copied_digest != digest {
        bail!("installed plugin source does not match the reviewed digest");
    }
    Ok(())
}

fn restart_previous(paths: &RegistryPaths, id: &str, stopped: bool) {
    if stopped {
        let _ = crate::execution::start_one(paths, id);
    }
}

fn preview(spec: &RegistrySpec, digest: &str, json: bool) -> Result<()> {
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "id": spec.id,
                "version": spec.version,
                "source": spec.source,
                "digest": digest,
                "manifest": spec.manifest,
            }))?
        );
    } else {
        println!(
            "id: {}\nversion: {}\nsource: {} @ {} ({})\ndigest: {}\nstartup: {:?}\nactions: {:?}",
            spec.id,
            spec.version,
            spec.source.repository,
            spec.source.commit,
            spec.source.tag,
            digest,
            spec.manifest.startup,
            spec.manifest.actions
        );
    }
    Ok(())
}

fn installed_path(paths: &RegistryPaths, id: &str, version: &str) -> PathBuf {
    paths.config.join("sources").join(id).join(version)
}
