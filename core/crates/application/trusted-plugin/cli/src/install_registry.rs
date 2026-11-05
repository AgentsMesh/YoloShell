use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Component, Path};
use yolo_trusted_plugin_manifest::{SCHEMA_VERSION, TrustedPluginManifest};

pub(crate) const DEFAULT_REGISTRY: &str = "https://github.com/AgentsMesh/YoloShellPlugins.git";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RegistrySpec {
    pub schema_version: u16,
    pub id: String,
    pub version: String,
    pub name: String,
    pub description: String,
    pub source: RegistrySource,
    pub manifest: TrustedPluginManifest,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RegistrySource {
    pub repository: String,
    pub path: String,
    pub tag: String,
    pub commit: String,
}

pub(crate) fn load_spec(
    registry: &Path,
    id: &str,
    requested: Option<&str>,
) -> Result<RegistrySpec> {
    if id.starts_with("yolo.") || id == "web" {
        bail!("the web client plugin id is web.client; the yolo. prefix is reserved");
    }
    if !is_plugin_id(id) {
        bail!("invalid plugin id {id:?}; use a dotted lowercase id such as web.client");
    }
    let directory = registry.join("Specs").join(id);
    let version = match requested {
        Some(version) => {
            validate_version(version)?;
            version.to_owned()
        }
        None => latest_version(&directory, id)?,
    };
    let path = directory.join(format!("{version}.json"));
    let bytes = std::fs::read(&path)
        .with_context(|| format!("read plugin registry spec {}", path.display()))?;
    let spec: RegistrySpec = serde_json::from_slice(&bytes)
        .with_context(|| format!("parse plugin registry spec {}", path.display()))?;
    validate(&spec, id, &version)?;
    Ok(spec)
}

fn latest_version(directory: &Path, id: &str) -> Result<String> {
    let entries = std::fs::read_dir(directory)
        .with_context(|| format!("plugin {id} is not present in the registry"))?;
    let mut versions = Vec::new();
    for entry in entries {
        let path = entry?.path();
        if path.extension().and_then(|value| value.to_str()) != Some("json") {
            continue;
        }
        let Some(version) = path.file_stem().and_then(|value| value.to_str()) else {
            continue;
        };
        if validate_version(version).is_ok() {
            versions.push(version.to_owned());
        }
    }
    versions.sort_by_key(|version| version_key(version));
    versions
        .pop()
        .ok_or_else(|| anyhow::anyhow!("plugin {id} has no version specs"))
}

fn validate(spec: &RegistrySpec, id: &str, version: &str) -> Result<()> {
    if spec.schema_version != 1 {
        bail!("registry spec schema_version must be 1");
    }
    if spec.id != id || spec.version != version {
        bail!("registry spec identity does not match {id} {version}");
    }
    if spec.name.is_empty() || spec.description.is_empty() {
        bail!("registry spec name and description must be non-empty");
    }
    validate_source(&spec.source)?;
    if spec.manifest.schema_version != SCHEMA_VERSION
        || spec.manifest.id != spec.id
        || spec.manifest.version != spec.version
        || spec.manifest.name != spec.name
        || spec.manifest.description.as_deref() != Some(spec.description.as_str())
    {
        bail!("registry manifest does not match its spec fields");
    }
    Ok(())
}

fn validate_source(source: &RegistrySource) -> Result<()> {
    if source.repository.trim().is_empty()
        || source.tag.trim().is_empty()
        || source.repository.chars().any(char::is_control)
        || source.tag.chars().any(char::is_control)
    {
        bail!("registry source repository and tag must be non-empty");
    }
    if !is_commit(&source.commit) {
        bail!("registry source commit must be 40 lowercase hex characters");
    }
    let path = Path::new(&source.path);
    if source.path.is_empty()
        || path.is_absolute()
        || path.components().any(|component| {
            matches!(component, Component::ParentDir | Component::RootDir | Component::Prefix(_))
        })
    {
        bail!("registry source path must be relative and must not contain ..");
    }
    Ok(())
}

fn is_plugin_id(id: &str) -> bool {
    !id.starts_with("yolo.")
        && id.contains('.')
        && id.split('.').all(|part| {
            !part.is_empty()
                && part
                    .chars()
                    .all(|character| character.is_ascii_lowercase()
                        || character.is_ascii_digit()
                        || character == '-')
        })
}

fn validate_version(version: &str) -> Result<()> {
    if version.split('.').count() != 3
        || version
            .split('.')
            .any(|part| part.is_empty() || part.parse::<u64>().is_err())
    {
        bail!("plugin version must be semver such as 0.1.0");
    }
    Ok(())
}

fn version_key(version: &str) -> (u64, u64, u64) {
    let mut parts = version.split('.').map(|part| part.parse().unwrap_or(0));
    (
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
    )
}

fn is_commit(value: &str) -> bool {
    value.len() == 40
        && value
            .chars()
            .all(|character| character.is_ascii_hexdigit() && !character.is_ascii_uppercase())
}

#[cfg(test)]
#[path = "../unit_tests/install.rs"]
mod install_tests;

