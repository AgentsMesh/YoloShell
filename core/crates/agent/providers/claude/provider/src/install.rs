//! Claude Code hook installation profile. The installation engine owns the
//! filesystem and merge protocol; this module owns only the provider's data.
use crate::{DISPLAY_NAME, INSTALL_TOKEN};
use yolo_claude::claude::constants::{DATA_HOME_ENV, DEFAULT_DATA_HOME_DIR};
use yolo_hook_installation_contract::{HookEventSpec as Event, HookInstallSpec};

const FILE_NAME: &str = "settings.json";
const HOOK_ARGUMENTS: &str = "provider-hook --provider claude";
const MANAGED_HOOK_COMMANDS: &[&str] = &["provider-hook --provider claude", "claude-hook"];
const DISABLED: &str = "disableAllHooks";
const MANAGED_ONLY: &str = "allowManagedHooksOnly";

const EVENTS: &[Event] = &[
    Event::new(yolo_claude_hook_contract::CLAUDE_SESSION_START_EVENT),
    Event::new(yolo_claude_hook_contract::CLAUDE_SESSION_END_EVENT),
    Event::new(yolo_claude_hook_contract::CLAUDE_USER_PROMPT_EVENT),
    Event::new(yolo_claude_hook_contract::CLAUDE_STOP_EVENT),
    Event::new(yolo_claude_hook_contract::CLAUDE_CWD_CHANGED_EVENT),
    Event::new(yolo_claude_hook_contract::CLAUDE_NOTIFICATION_EVENT),
    Event::new(yolo_claude_hook_contract::CLAUDE_PRE_TOOL_USE_EVENT),
    Event::with_timeout(
        yolo_claude_hook_contract::CLAUDE_PERMISSION_REQUEST_EVENT,
        600,
    ),
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
    restricted_keys: &[DISABLED, MANAGED_ONLY],
    disabled_paths: &[],
    requires_provider_trust: false,
};

pub fn install_spec() -> &'static HookInstallSpec {
    &INSTALL_SPEC
}
