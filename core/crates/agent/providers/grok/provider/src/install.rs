//! Grok Build hook installation profile. The installation engine owns the
//! filesystem and merge protocol; this module owns only the provider's data.
use crate::DISPLAY_NAME;
use yolo_grok::grok::constants::{DATA_HOME_DIR, DATA_HOME_ENV};
use yolo_hook_installation_contract::{HookEventSpec as Event, HookInstallSpec};

const FILE_NAME: &str = "yolo.json";
const HOOK_ARGUMENTS: &str = "provider-hook --provider grok";
const MANAGED_HOOK_COMMANDS: &[&str] = &["provider-hook --provider grok", "grok-hook"];

const EVENTS: &[Event] = &[
    Event::new(yolo_grok_hooks::EVENT_SESSION_START),
    Event::new(yolo_grok_hooks::EVENT_SESSION_END),
    Event::new(yolo_grok_hooks::EVENT_USER_PROMPT_SUBMIT),
    Event::new(yolo_grok_hooks::EVENT_STOP),
    Event::new(yolo_grok_hooks::EVENT_STOP_CANCELLED),
    Event::new(yolo_grok_hooks::EVENT_STOP_FAILURE),
    Event::new(yolo_grok_hooks::EVENT_PRE_TOOL_USE),
    Event::new(yolo_grok_hooks::EVENT_POST_TOOL_USE),
    Event::new(yolo_grok_hooks::EVENT_POST_TOOL_USE_FAILURE),
    Event::new(yolo_grok_hooks::EVENT_PERMISSION_DENIED),
    Event::new(yolo_grok_hooks::EVENT_NOTIFICATION),
    Event::new(yolo_grok_hooks::EVENT_PRE_COMPACT),
    Event::new(yolo_grok_hooks::EVENT_POST_COMPACT),
    Event::new(yolo_grok_hooks::EVENT_SUBAGENT_START),
    Event::new(yolo_grok_hooks::EVENT_SUBAGENT_STOP),
];

pub const INSTALL_SPEC: HookInstallSpec = HookInstallSpec {
    provider: yolo_grok_hooks::PROVIDER,
    display_name: DISPLAY_NAME,
    file_name: FILE_NAME,
    subdirectory: yolo_grok_hooks::HOOKS_DIRECTORY,
    environment_name: DATA_HOME_ENV,
    directory_name: DATA_HOME_DIR,
    hook_arguments: HOOK_ARGUMENTS,
    managed_hook_commands: MANAGED_HOOK_COMMANDS,
    events: EVENTS,
    restricted_keys: &[],
    disabled_paths: &[],
    requires_provider_trust: true,
};

pub fn install_spec() -> &'static HookInstallSpec {
    &INSTALL_SPEC
}
