use std::path::Path;
use yolo_paths_core::platform::platform_path;

const MACOS_TEMP_ALIAS: &str = "/tmp/yolo/session";
const MACOS_TEMP_TARGET: &str = "/private/tmp/yolo/session";
const MACOS_VAR_ALIAS: &str = "/var/run/yolo";
const MACOS_VAR_TARGET: &str = "/private/var/run/yolo";
const UNALIASED_PATHS: &[&str] = &[
    "/Users/example/.yolo",
    "/tmp-yolo/session",
    "/variable/run/yolo",
];

#[cfg(target_os = "macos")]
#[test]
fn macos_system_aliases_are_resolved_by_component() {
    assert_eq!(
        platform_path(Path::new(MACOS_TEMP_ALIAS)),
        Path::new(MACOS_TEMP_TARGET)
    );
    assert_eq!(
        platform_path(Path::new(MACOS_VAR_ALIAS)),
        Path::new(MACOS_VAR_TARGET)
    );
}

#[test]
fn paths_outside_platform_aliases_are_unchanged() {
    for path in UNALIASED_PATHS {
        let path = Path::new(path);
        assert_eq!(platform_path(path), path);
    }
}
