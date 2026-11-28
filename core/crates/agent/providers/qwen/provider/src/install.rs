//! Qwen Code hook installation profile. The installation engine owns the
//! filesystem and merge protocol; this module owns only the provider's data.
use crate::DISPLAY_NAME;
use yolo_hook_installation_contract::{HookEventSpec as Event, HookInstallSpec};
use yolo_qwen::qwen::constants::{DATA_HOME_ENV, DEFAULT_DATA_HOME_DIR};

const FILE_NAME: &str = "settings.json";
const HOOK_ARGUMENTS: &str = "provider-hook --provider qwen";
const MANAGED_HOOK_COMMANDS: &[&str] = &["qwen-hook", "provider-hook --provider qwen"];
const DISABLED: &str = "disableAllHooks";

const EVENTS: &[Event] = &[
    Event::new(yolo_qwen_hooks::EVENT_SESSION_START),
    Event::new(yolo_qwen_hooks::EVENT_USER_PROMPT_SUBMIT),
    Event::new(yolo_qwen_hooks::EVENT_PRE_TOOL_USE),
    Event::new(yolo_qwen_hooks::EVENT_POST_TOOL_USE),
    Event::new(yolo_qwen_hooks::EVENT_POST_TOOL_USE_FAILURE),
    Event::new(yolo_qwen_hooks::EVENT_POST_TOOL_BATCH),
    Event::new(yolo_qwen_hooks::EVENT_NOTIFICATION),
    Event::new(yolo_qwen_hooks::EVENT_PERMISSION_REQUEST),
    Event::new(yolo_qwen_hooks::EVENT_PERMISSION_DENIED),
    Event::new(yolo_qwen_hooks::EVENT_SUBAGENT_START),
    Event::new(yolo_qwen_hooks::EVENT_SUBAGENT_STOP),
    Event::new(yolo_qwen_hooks::EVENT_PRE_COMPACT),
    Event::new(yolo_qwen_hooks::EVENT_POST_COMPACT),
    Event::new(yolo_qwen_hooks::EVENT_SESSION_DELETE),
    Event::new(yolo_qwen_hooks::EVENT_SESSION_END),
];

pub const INSTALL_SPEC: HookInstallSpec = HookInstallSpec {
    provider: yolo_qwen_hooks::PROVIDER,
    display_name: DISPLAY_NAME,
    file_name: FILE_NAME,
    subdirectory: "",
    environment_name: DATA_HOME_ENV,
    directory_name: DEFAULT_DATA_HOME_DIR,
    hook_arguments: HOOK_ARGUMENTS,
    managed_hook_commands: MANAGED_HOOK_COMMANDS,
    events: EVENTS,
    restricted_keys: &[DISABLED],
    disabled_paths: &[],
    requires_provider_trust: false,
};

pub fn install_spec() -> &'static HookInstallSpec {
    &INSTALL_SPEC
}
