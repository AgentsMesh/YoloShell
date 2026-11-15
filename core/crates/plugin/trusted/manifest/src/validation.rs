use std::path::{Path, PathBuf};

use yolo_plugin_api::valid_action_id;

use crate::{MANIFEST_FILE, MAX_MANIFEST_BYTES, SCHEMA_VERSION, TrustedPluginManifest};

pub const MAX_ACTION_COUNT: usize = 64;
pub const MAX_ARGUMENTS: usize = 64;
pub const MAX_ARGUMENT_BYTES: usize = 16 * 1024;
pub const MAX_IDENTIFIER_BYTES: usize = 128;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ManifestError {
    Io,
    Schema,
    Identifier,
    Platform,
    Command,
    Action,
    Api,
    TooLarge,
}

impl std::fmt::Display for ManifestError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Io => "could not read plugin manifest",
            Self::Schema => "unsupported plugin manifest schema",
            Self::Identifier => "invalid plugin identity",
            Self::Platform => "invalid plugin platform list",
            Self::Command => "invalid plugin command",
            Self::Action => "invalid plugin action",
            Self::Api => "invalid plugin API contract",
            Self::TooLarge => "plugin manifest is too large",
        })
    }
}

impl std::error::Error for ManifestError {}

pub fn load(root: &Path) -> Result<TrustedPluginManifest, ManifestError> {
    let path = manifest_path(root);
    if std::fs::metadata(&path)
        .map_err(|_| ManifestError::Io)?
        .len()
        > MAX_MANIFEST_BYTES
    {
        return Err(ManifestError::TooLarge);
    }
    let bytes = std::fs::read(&path).map_err(|_| ManifestError::Io)?;
    parse(&bytes)
}

pub fn parse(bytes: &[u8]) -> Result<TrustedPluginManifest, ManifestError> {
    if bytes.len() as u64 > MAX_MANIFEST_BYTES {
        return Err(ManifestError::TooLarge);
    }
    let manifest: TrustedPluginManifest =
        serde_json::from_slice(bytes).map_err(|_| ManifestError::Schema)?;
    validate(&manifest)?;
    Ok(manifest)
}

fn validate(manifest: &TrustedPluginManifest) -> Result<(), ManifestError> {
    if manifest.schema_version != SCHEMA_VERSION
        || manifest.name.trim().is_empty()
        || manifest.version.trim().is_empty()
        || manifest
            .web
            .as_ref()
            .is_some_and(|web| web.title.trim().is_empty())
    {
        return Err(ManifestError::Schema);
    }
    if !valid_plugin_id(&manifest.id) || manifest.id.starts_with("yolo.") {
        return Err(ManifestError::Identifier);
    }
    validate_platforms(&manifest.platforms)?;
    if manifest.startup.is_none() && manifest.actions.is_empty() {
        return Err(ManifestError::Command);
    }
    if manifest.web.is_some() && manifest.startup.is_none() {
        return Err(ManifestError::Command);
    }
    if let Some(startup) = &manifest.startup {
        validate_argv(&startup.command)?;
    }
    if manifest.actions.len() > MAX_ACTION_COUNT {
        return Err(ManifestError::Action);
    }
    for action in &manifest.actions {
        if !valid_action_id(&action.id)
            || action.title.trim().is_empty()
            || manifest
                .actions
                .iter()
                .filter(|other| other.id == action.id)
                .count()
                > 1
        {
            return Err(ManifestError::Action);
        }
        validate_argv(&action.command)?;
    }
    let api = &manifest.api;
    if api.protocol_version == u16::MIN
        || api.required_methods.is_empty()
        || !unique(&api.required_methods)
        || !unique(&api.optional_methods)
        || api
            .required_methods
            .iter()
            .any(|method| api.optional_methods.contains(method))
        || api
            .features
            .iter()
            .flat_map(|feature| &feature.methods)
            .any(|method| {
                !api.required_methods.contains(method) && !api.optional_methods.contains(method)
            })
    {
        return Err(ManifestError::Api);
    }
    Ok(())
}

fn validate_platforms(platforms: &[yolo_plugin_api::Platform]) -> Result<(), ManifestError> {
    if platforms.is_empty()
        || platforms
            .iter()
            .enumerate()
            .any(|(index, platform)| platforms[..index].contains(platform))
    {
        return Err(ManifestError::Platform);
    }
    Ok(())
}

fn validate_argv(argv: &[String]) -> Result<(), ManifestError> {
    if argv.is_empty()
        || argv[0].is_empty()
        || argv.len() > MAX_ARGUMENTS
        || argv.iter().any(|argument| argument.contains('\0'))
        || argv.iter().map(String::len).sum::<usize>() > MAX_ARGUMENT_BYTES
    {
        return Err(ManifestError::Command);
    }
    Ok(())
}

fn valid_plugin_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_IDENTIFIER_BYTES
        && value.contains('.')
        && value.split('.').all(|part| {
            !part.is_empty()
                && part
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        })
}

fn unique<T: PartialEq>(values: &[T]) -> bool {
    values
        .iter()
        .enumerate()
        .all(|(index, value)| !values[..index].contains(value))
}

pub fn manifest_path(root: &Path) -> PathBuf {
    root.join(MANIFEST_FILE)
}
