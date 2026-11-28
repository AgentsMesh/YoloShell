use std::path::{Path, PathBuf};

const DEFAULT_INSTALL_DATA: &str = ".local/share";
const VERSIONS: &str = "claude/versions";
const XDG_DATA_HOME: &str = "XDG_DATA_HOME";

/// Native Claude installs keep each release in the account's data directory.
/// Select by the kernel-observed path, including older versions still running.
pub fn native_version(path: &Path) -> Option<&str> {
    let name = path.file_name()?.to_str()?;
    let parent = path.parent()?;
    if !parent.ends_with(VERSIONS) || !release_name(name) {
        return None;
    }
    let matches = |base: PathBuf| base.is_absolute() && parent == base.join(VERSIONS);
    let xdg_data = std::env::var_os(XDG_DATA_HOME).map(PathBuf::from);
    let default_data = std::env::home_dir().map(|home| home.join(DEFAULT_INSTALL_DATA));
    (xdg_data.is_some_and(matches) || default_data.is_some_and(matches)).then_some(name)
}

fn release_name(name: &str) -> bool {
    let core = match name.split_once(['-', '+']) {
        Some((core, extra))
            if !extra.is_empty()
                && extra
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'.' | b'-' | b'+')) =>
        {
            core
        }
        Some(_) => return false,
        None => name,
    };
    let mut parts = core.split('.');
    let numeric = |part: &str| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit());
    parts.next().is_some_and(numeric)
        && parts.next().is_some_and(numeric)
        && parts.next().is_some_and(numeric)
        && parts.next().is_none()
}
