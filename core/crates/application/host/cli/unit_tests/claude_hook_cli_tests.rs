use super::*;
use yolo_launch_contract::EXECUTABLE_NAME;
use yolo_runtime::cli::constants::{HOME_ARGUMENT, PROVIDER_ARGUMENT, PROVIDER_HOOK_COMMAND};

const CALLBACK_HOME: &str = "/tmp/yolo-hook-cli";
const CLAUDE: &str = yolo_claude_provider::INSTALL_TOKEN;
#[path = "../test_support/hook_constants.rs"]
mod constants;
use constants::RETIRED_CAPABILITY_ARGUMENT;

#[test]
fn native_callback_accepts_runtime_home_without_launch_credentials() {
    let parsed = Cli::try_parse_from([
        EXECUTABLE_NAME,
        PROVIDER_HOOK_COMMAND,
        PROVIDER_ARGUMENT,
        CLAUDE,
        HOME_ARGUMENT,
        CALLBACK_HOME,
    ])
    .unwrap();
    assert!(!needs_detached_session(&parsed.command));
    assert!(!command_emits_logs(&parsed.command));
    let Command::ProviderHook { provider, home } = parsed.command else {
        panic!("expected provider hook callback");
    };
    assert_eq!(home, PathBuf::from(CALLBACK_HOME));
    assert_eq!(
        crate::provider_hook::client(&provider),
        Some(yolo_runtime::agent::HookClient::ClaudeNative)
    );
}

#[test]
fn native_callback_rejects_missing_home_and_retired_launch_credentials() {
    assert!(
        Cli::try_parse_from([
            EXECUTABLE_NAME,
            PROVIDER_HOOK_COMMAND,
            PROVIDER_ARGUMENT,
            CLAUDE,
        ])
        .is_err()
    );
    assert!(
        Cli::try_parse_from([
            EXECUTABLE_NAME,
            PROVIDER_HOOK_COMMAND,
            PROVIDER_ARGUMENT,
            CLAUDE,
            HOME_ARGUMENT,
            CALLBACK_HOME,
            RETIRED_CAPABILITY_ARGUMENT,
            &uuid::Uuid::new_v4().to_string(),
        ])
        .is_err()
    );
}
