use super::*;
use crate::packaged_claude_test_support::*;
use serde_json::json;
use yolo_hook_installation::{inspect, install};
use yolo_hook_installation_catalog as catalog;
#[path = "../test_support/installed_hook.rs"]
mod fixture;
use fixture::*;

const HOOK_EXECUTABLE_ENV: &str = "YOLO_HOOK_EXECUTABLE";
const HOOK_HOME_ENV: &str = "YOLO_HOOK_HOME";
const PERMISSION_MODE: u32 = 0o700;

#[test]
fn actual_installer_commands_execute_unmodified_for_both_providers() {
    use std::os::unix::fs::PermissionsExt;
    for provider in [&catalog::CLAUDE, &catalog::CODEX] {
        let root = tempfile::tempdir().unwrap();
        std::fs::set_permissions(
            root.path(),
            std::fs::Permissions::from_mode(PERMISSION_MODE),
        )
        .unwrap();
        let helper = executable(root.path(), HELPER_FILE_NAME, &callback_script(provider));
        let capture = root.path().join(CAPTURE_FILE_NAME);
        let preview = inspect(provider, root.path()).unwrap();
        install(provider, root.path(), &preview.revision).unwrap();
        let settings = load(&root.path().join(provider.file_name)).unwrap();
        let hook = &settings.hooks[crate::fake_agent_constants::PACKAGED_SESSION_START_EVENT][0];
        assert_eq!(
            hook.command,
            yolo_hook_installation::callback_command(provider)
        );
        let mut command = shell_command(hook);
        command
            .env(HOOK_EXECUTABLE_ENV, helper)
            .env(HOOK_HOME_ENV, &capture);
        let payload = json!({crate::fake_agent_constants::PACKAGED_HOOK_SESSION_FIELD: SESSION_ID});
        run_command(command, &payload).unwrap();
        assert_eq!(
            serde_json::from_slice::<Value>(&std::fs::read(capture).unwrap()).unwrap(),
            payload
        );
    }
}

#[test]
fn callback_fixture_rejects_inexact_roles_provider_commands_and_arguments_before_writing() {
    use std::ffi::OsString;
    use yolo_runtime::cli::{ProcessRole, constants};
    for provider in [&catalog::CLAUDE, &catalog::CODEX] {
        let root = tempfile::tempdir().unwrap();
        let capture = root.path().join(CAPTURE_FILE_NAME);
        let helper = executable(root.path(), HELPER_FILE_NAME, &callback_script(provider));
        let suffix = vec![
            OsString::from(hook_command(provider)),
            constants::HOME_ARGUMENT.into(),
            capture.clone().into_os_string(),
        ];
        let other = if provider == &catalog::CLAUDE {
            &catalog::CODEX
        } else {
            &catalog::CLAUDE
        };
        let prefix = ProcessRole::Host.arguments();
        let cases = [
            suffix.clone(),
            [vec![prefix[0].into(), WRONG_ROLE.into()], suffix.clone()].concat(),
            ProcessRole::Host.prepend([
                hook_command(other).into(),
                constants::HOME_ARGUMENT.into(),
                capture.clone().into_os_string(),
            ]),
            ProcessRole::Host.prepend([
                hook_command(provider).into(),
                WRONG_HOME_OPTION.into(),
                capture.clone().into_os_string(),
            ]),
            ProcessRole::Host.prepend([
                hook_command(provider).into(),
                constants::HOME_ARGUMENT.into(),
            ]),
            ProcessRole::Host.prepend(suffix.into_iter().chain([EXTRA_ARGUMENT.into()])),
        ];
        for arguments in cases {
            let mut command = std::process::Command::new(&helper);
            command.args(arguments);
            assert!(run_command(command, &json!({})).is_err());
            assert!(!capture.exists());
        }
    }
}

#[test]
fn installed_command_is_a_noop_without_yolo_callback_context() {
    for provider in [&catalog::CLAUDE, &catalog::CODEX] {
        let hook = HookCommand {
            command: yolo_hook_installation::callback_command(provider),
        };
        let mut command = shell_command(&hook);
        command
            .env_remove(HOOK_EXECUTABLE_ENV)
            .env_remove(HOOK_HOME_ENV);
        run_command(command, &json!({})).unwrap();
    }
}

#[test]
fn disabled_provider_hooks_do_not_execute_user_configuration() {
    for key in RESTRICTIONS {
        let mut configuration = valid_settings(SHELL);
        configuration[*key] = json!(true);
        assert!(parse(&configuration).unwrap().hooks.is_empty());
    }
}
