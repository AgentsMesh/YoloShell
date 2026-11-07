use yolo_hook_installation::HookInstallSpec;
use yolo_hook_installation_catalog as catalog;

pub fn launch_values(provider: &HookInstallSpec) -> (&'static str, &'static str, &'static str) {
    if provider == &catalog::CLAUDE {
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
