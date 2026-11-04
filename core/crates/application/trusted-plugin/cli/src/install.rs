use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};
use yolo_trusted_plugin_manifest::load;
use yolo_trusted_plugin_registry::{Registry, RegistryPaths, source_digest};

use crate::install_git;
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

    let target = installed_path(paths, &spec.id, &spec.version);
    if target.exists() {
        bail!(
            "plugin {} {} is already installed at {}",
            spec.id,
            spec.version,
            target.display()
        );
    }
    let result = commit_source(&target, &prepared, &digest)
        .and_then(|()| link(paths, &target, &digest));
    if result.is_err() {
        let _ = std::fs::remove_dir_all(&target);
    }
    result?;
    if !json {
        println!("installed {} {} at {}", spec.id, spec.version, target.display());
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

fn link(paths: &RegistryPaths, source: &Path, digest: &str) -> Result<()> {
    let mut registry = Registry::load_locked(paths)?;
    registry.link_approved(source, digest, paths)?;
    Ok(())
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
