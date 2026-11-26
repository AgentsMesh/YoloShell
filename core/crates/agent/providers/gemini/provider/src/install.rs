//! Gemini CLI hook installation profile. The installation engine owns the
//! filesystem and merge protocol; this module owns only the provider's data.
use crate::DISPLAY_NAME;
use yolo_gemini::gemini::constants::{DATA_HOME_DIR, DATA_HOME_ENV};
use yolo_hook_installation_contract::{HookEventSpec as Event, HookInstallSpec};

const FILE_NAME: &str = "settings.json";
const HOOK_ARGUMENTS: &str = "provider-hook --provider gemini";
const MANAGED_HOOK_COMMANDS: &[&str] = &["provider-hook --provider gemini", "gemini-hook"];
const DISABLED_HOOKS_PATH: &[&str] = &["hooksConfig", "enabled"];

const EVENTS: &[Event] = &[
    Event::new(yolo_gemini_hooks::EVENT_SESSION_START),
    Event::new(yolo_gemini_hooks::EVENT_BEFORE_AGENT),
    Event::new(yolo_gemini_hooks::EVENT_AFTER_AGENT),
    Event::new(yolo_gemini_hooks::EVENT_BEFORE_TOOL),
    Event::new(yolo_gemini_hooks::EVENT_AFTER_TOOL),
    Event::new(yolo_gemini_hooks::EVENT_NOTIFICATION),
    Event::new(yolo_gemini_hooks::EVENT_SESSION_END),
];

pub const INSTALL_SPEC: HookInstallSpec = HookInstallSpec {
    provider: yolo_gemini_hooks::PROVIDER,
    display_name: DISPLAY_NAME,
    file_name: FILE_NAME,
    subdirectory: "",
    environment_name: DATA_HOME_ENV,
    directory_name: DATA_HOME_DIR,
    hook_arguments: HOOK_ARGUMENTS,
    managed_hook_commands: MANAGED_HOOK_COMMANDS,
    events: EVENTS,
    restricted_keys: &[],
    disabled_paths: &[DISABLED_HOOKS_PATH],
    requires_provider_trust: true,
};

pub fn install_spec() -> &'static HookInstallSpec {
    &INSTALL_SPEC
}
