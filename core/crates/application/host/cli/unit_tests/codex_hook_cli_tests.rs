use super::*;
use yolo_launch_contract::EXECUTABLE_NAME;
use yolo_runtime::cli::constants::{HOME_ARGUMENT, PROVIDER_ARGUMENT, PROVIDER_HOOK_COMMAND};

const TEST_HOME: &str = "/tmp/yolo-codex-hook-cli";
const CODEX: &str = yolo_codex_provider::INSTALL_TOKEN;

#[test]
fn hidden_provider_hook_requires_provider_and_home() {
    assert!(Cli::try_parse_from([EXECUTABLE_NAME, PROVIDER_HOOK_COMMAND]).is_err());
    assert!(
        Cli::try_parse_from([
            EXECUTABLE_NAME,
            PROVIDER_HOOK_COMMAND,
            PROVIDER_ARGUMENT,
            CODEX,
        ])
        .is_err()
    );
}

#[test]
fn hidden_provider_hook_accepts_explicit_home() {
    let parsed = Cli::try_parse_from([
        EXECUTABLE_NAME,
        PROVIDER_HOOK_COMMAND,
        PROVIDER_ARGUMENT,
        CODEX,
        HOME_ARGUMENT,
        TEST_HOME,
    ])
    .unwrap();

    let Command::ProviderHook { provider, home } = parsed.command else {
        panic!("expected provider hook command");
    };
    assert_eq!(provider, CODEX);
    assert_eq!(home, PathBuf::from(TEST_HOME));
    assert_eq!(
        crate::provider_hook::client(&provider),
        Some(yolo_runtime::agent::HookClient::CodexNative)
    );
}
