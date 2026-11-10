//! Provider availability facts derived from the Host's launch environment.
//! The Hub projects these into `list_providers`; Web and other clients never
//! inspect the Host filesystem themselves.

use std::path::Path;
use yolo_agent_contract::PROVIDER_CAPABILITY_PREFIX;

const EXECUTABLE_MODE_MASK: u32 = 0o111;

pub fn provider_capabilities() -> Vec<String> {
    yolo_agent_providers::builtin_launch_policies()
        .entries()
        .filter(|(_, command)| executable_available(command))
        .map(|(kind, _)| format!("{PROVIDER_CAPABILITY_PREFIX}{}", kind.as_str()))
        .collect()
}

fn executable_available(command: &str) -> bool {
    if command.contains('/') {
        return executable_file(Path::new(command));
    }
    std::env::var_os("PATH").is_some_and(|paths| {
        std::env::split_paths(&paths)
            .any(|directory| executable_file(&directory.join(command)))
    })
}

#[cfg(unix)]
fn executable_file(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    path.metadata().is_ok_and(|metadata| {
        metadata.is_file() && metadata.permissions().mode() & EXECUTABLE_MODE_MASK != 0
    })
}

#[cfg(not(unix))]
fn executable_file(path: &Path) -> bool {
    path.is_file()
}

#[cfg(test)]
#[path = "../unit_tests/lib.rs"]
mod unit_tests;
