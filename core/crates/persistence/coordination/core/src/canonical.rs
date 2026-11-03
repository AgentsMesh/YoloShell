use anyhow::{Context, Result};
use std::ffi::OsString;
use std::os::fd::AsRawFd;
use std::os::unix::ffi::OsStringExt;
use std::path::{Path, PathBuf};

use super::constants::*;

#[cfg(target_os = "macos")]
pub fn canonical_logical_path(path: &Path) -> Result<PathBuf> {
    let platform = yolo_paths_core::platform::platform_path(path);
    let (existing, suffix) = deepest_existing_path(platform)?;
    let canonical = std::fs::canonicalize(&existing)
        .with_context(|| format!("canonicalize coordination path {}", existing.display()))?;
    let physical = physical_firmlink_path(&canonical);
    let case_sensitive = volume_is_case_sensitive(&physical)?;
    let mut canonical = normalize_existing(physical, case_sensitive);
    for component in suffix.into_iter().rev() {
        canonical.push(normalize_missing(component, case_sensitive)?);
    }
    Ok(canonical)
}

#[cfg(target_os = "macos")]
fn normalize_existing(path: PathBuf, case_sensitive: bool) -> PathBuf {
    if case_sensitive {
        return path;
    }
    let mut bytes = path.into_os_string().into_vec();
    bytes.make_ascii_lowercase();
    PathBuf::from(OsString::from_vec(bytes))
}

#[cfg(not(target_os = "macos"))]
pub fn canonical_logical_path(path: &Path) -> Result<PathBuf> {
    let (existing, suffix) = deepest_existing_path(yolo_paths_core::platform::platform_path(path))?;
    let mut canonical = std::fs::canonicalize(existing)?;
    for component in suffix.into_iter().rev() {
        canonical.push(component);
    }
    Ok(canonical)
}

fn deepest_existing_path(mut path: PathBuf) -> Result<(PathBuf, Vec<OsString>)> {
    let mut suffix = Vec::new();
    loop {
        match std::fs::canonicalize(&path) {
            Ok(_) => return Ok((path, suffix)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                let name = path
                    .file_name()
                    .context("coordination path has no existing ancestor")?;
                suffix.push(name.to_os_string());
                anyhow::ensure!(path.pop(), "coordination path has no existing ancestor");
            }
            Err(error) => {
                return Err(error)
                    .with_context(|| format!("resolve coordination path {}", path.display()));
            }
        }
    }
}

#[cfg(target_os = "macos")]
fn physical_firmlink_path(path: &Path) -> PathBuf {
    for (alias, physical) in MACOS_FIRMLINK_ALIASES {
        if let Ok(remainder) = path.strip_prefix(alias) {
            return if remainder.as_os_str().is_empty() {
                PathBuf::from(physical)
            } else {
                Path::new(physical).join(remainder)
            };
        }
    }
    path.to_path_buf()
}

#[cfg(target_os = "macos")]
fn volume_is_case_sensitive(path: &Path) -> Result<bool> {
    let metadata = std::fs::metadata(path)
        .with_context(|| format!("inspect coordination ancestor {}", path.display()))?;
    let probe = if metadata.is_dir() {
        path
    } else {
        path.parent()
            .context("coordination volume probe has no parent")?
    };
    let file = std::fs::File::open(probe)
        .with_context(|| format!("open coordination ancestor {}", probe.display()))?;
    let value = unsafe { libc::fpathconf(file.as_raw_fd(), libc::_PC_CASE_SENSITIVE) };
    validate_case_sensitivity(value)
}

#[cfg(target_os = "macos")]
fn validate_case_sensitivity(value: libc::c_long) -> Result<bool> {
    anyhow::ensure!(
        value == CASE_INSENSITIVE_VOLUME || value == CASE_SENSITIVE_VOLUME,
        "query coordination volume case sensitivity: {}",
        std::io::Error::last_os_error()
    );
    Ok(value == CASE_SENSITIVE_VOLUME)
}

#[cfg(target_os = "macos")]
fn normalize_missing(component: OsString, case_sensitive: bool) -> Result<OsString> {
    if case_sensitive {
        return Ok(component);
    }
    let mut bytes = component.into_vec();
    anyhow::ensure!(bytes.is_ascii(), CASE_INSENSITIVE_SUFFIX_ERROR);
    bytes.make_ascii_lowercase();
    Ok(OsString::from_vec(bytes))
}

#[cfg(all(test, target_os = "macos"))]
#[path = "../unit_tests/canonical_tests.rs"]
mod tests;

#[cfg(all(test, target_os = "linux"))]
#[path = "../unit_tests/canonical_linux_tests.rs"]
mod linux_tests;
