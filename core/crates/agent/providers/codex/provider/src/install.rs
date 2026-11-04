//! Codex hook installation profile. The installation engine owns the
//! filesystem and merge protocol; this module owns only the provider's data.
use crate::{DISPLAY_NAME, INSTALL_TOKEN};
use yolo_codex::codex::constants::{DATA_HOME_ENV, DEFAULT_DATA_HOME_DIR};
use yolo_codex_hook_contract::{
    SESSION_END_HOOK_TIMEOUT_SECONDS, event_names,
};
use yolo_hook_installation_contract::{HookEventSpec as Event, HookInstallSpec};

const FILE_NAME: &str = "hooks.json";
const HOOK_ARGUMENTS: &str = "provider-hook --provider codex";
const MANAGED_HOOK_COMMANDS: &[&str] = &["provider-hook --provider codex", "codex-hook"];

const EVENTS: &[Event] = &[
    Event::new(event_names::SESSION_START),
    Event::with_timeout(event_names::SESSION_END, SESSION_END_HOOK_TIMEOUT_SECONDS),
    Event::new(event_names::USER_PROMPT_SUBMIT),
    Event::new(event_names::PRE_TOOL_USE),
    Event::new(event_names::PERMISSION_REQUEST),
    Event::new(event_names::POST_TOOL_USE),
    Event::new(event_names::PRE_COMPACT),
    Event::new(event_names::POST_COMPACT),
    Event::new(event_names::SUBAGENT_START),
    Event::new(event_names::SUBAGENT_STOP),
    Event::new(event_names::STOP),
];

pub const INSTALL_SPEC: HookInstallSpec = HookInstallSpec {
    provider: INSTALL_TOKEN,
    display_name: DISPLAY_NAME,
    file_name: FILE_NAME,
    subdirectory: "",
    environment_name: DATA_HOME_ENV,
    directory_name: DEFAULT_DATA_HOME_DIR,
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
