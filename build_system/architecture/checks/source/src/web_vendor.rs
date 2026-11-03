use std::path::Path;

const VENDOR_DIRECTORY: &str = "vendor";
const VENDOR_FILE_NAMES: &[&str] = &[
    "xterm.js",
    "xterm.css",
    "addon-fit.js",
    "LICENSE.xterm",
    "LICENSE.addon-fit",
    "manifest.json",
];

pub(crate) fn is_vendor_path(path: &Path) -> bool {
    path.ancestors().any(|ancestor| ancestor.ends_with(vendor_root()))
}

pub(crate) fn is_vendored_web_asset(path: &Path) -> bool {
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    if !VENDOR_FILE_NAMES.contains(&name) {
        return false;
    }
    path.parent()
        .is_some_and(|parent| parent.ends_with(vendor_root()))
}

fn vendor_root() -> std::path::PathBuf {
    Path::new("plugins")
        .join("web")
        .join("client")
        .join(VENDOR_DIRECTORY)
}
