use std::path::{Path, PathBuf};

#[cfg(target_os = "macos")]
const MACOS_SYSTEM_PATH_ALIASES: &[(&str, &str)] =
    &[("/tmp", "/private/tmp"), ("/var", "/private/var")];

#[cfg(target_os = "macos")]
pub fn platform_path(path: &Path) -> PathBuf {
    for (alias, target) in MACOS_SYSTEM_PATH_ALIASES {
        if let Ok(remainder) = path.strip_prefix(alias) {
            return Path::new(target).join(remainder);
        }
    }
    path.to_path_buf()
}

#[cfg(not(target_os = "macos"))]
pub fn platform_path(path: &Path) -> PathBuf {
    path.to_path_buf()
}
