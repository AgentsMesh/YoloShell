use yolo_hook_installation::HookInstallSpec;

pub fn launch_values(provider: &HookInstallSpec) -> (&'static str, &'static str, &'static str) {
    if provider == crate::support::claude_install_spec() {
        (
            yolo_runtime::agent::claude::constants::EXECUTABLE,
            crate::FAKE_CLAUDE_BINARY_RUNFILE,
            yolo_runtime::agent::claude::constants::DATA_HOME_ENV,
        )
    } else {
        (
            yolo_runtime::agent::codex::constants::EXECUTABLE,
            crate::FAKE_CODEX_BINARY_RUNFILE,
            yolo_runtime::agent::codex::constants::DATA_HOME_ENV,
        )
    }
}
