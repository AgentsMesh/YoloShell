use super::constants;
use super::test_support::{self, Fixture};
use crate::agent::{AgentKind, AgentRegistry, DetectionResult};
use std::fs;

const OVERRIDE_DIRECTORY: &str = "override-project";
const SHELL_DIRECTORY: &str = "shell-project";
const RELATIVE_DIRECTORY: &str = "relative-project";

#[test]
fn fresh_and_resumed_cwd_options_override_recorded_metadata() {
    let fixture = Fixture::new();
    fixture.write_main(test_support::SESSION_ID);
    let override_cwd = fixture.cwd.with_file_name(OVERRIDE_DIRECTORY);
    fs::create_dir(&override_cwd).unwrap();
    let override_value = override_cwd.to_str().unwrap();
    let forms = [
        vec![constants::CWD_ARGUMENT, override_value],
        vec![
            constants::RESUME_ARGUMENT,
            constants::CWD_ARGUMENT,
            override_value,
            test_support::SESSION_ID,
        ],
        vec![
            constants::CWD_ARGUMENT,
            override_value,
            constants::RESUME_ARGUMENT,
            test_support::SESSION_ID,
        ],
    ];
    for arguments in forms {
        let process = fixture.process(
            test_support::PROCESS_PID,
            test_support::PROCESS_START_TIME,
            &arguments,
            Some(&[test_support::SESSION_ID]),
        );
        let DetectionResult::BindingChanged(binding) =
            fixture.detector().detect(&[process]).unwrap()
        else {
            panic!("explicit Codex cwd must bind")
        };
        assert_eq!(
            binding.cwd,
            override_cwd.canonicalize().unwrap().display().to_string()
        );
        yolo_agent_providers::builtin_agent_registry()
            .validate_binding(&binding, &fixture.context())
            .unwrap();
    }
}

#[test]
fn recorded_provider_cwd_wins_without_an_explicit_override() {
    let fixture = Fixture::new();
    fixture.write_main(test_support::SESSION_ID);
    let shell_cwd = fixture.cwd.with_file_name(SHELL_DIRECTORY);
    fs::create_dir(&shell_cwd).unwrap();
    let mut process = fixture.process(
        test_support::PROCESS_PID,
        test_support::PROCESS_START_TIME,
        &[],
        Some(&[test_support::SESSION_ID]),
    );
    process.cwd = Some(shell_cwd);
    let DetectionResult::BindingChanged(binding) = fixture.detector().detect(&[process]).unwrap()
    else {
        panic!("associated Codex evidence must bind")
    };
    assert_eq!(
        binding.cwd,
        fixture.cwd.canonicalize().unwrap().display().to_string()
    );
}

#[test]
fn relative_cwd_override_resolves_from_the_os_process_cwd() {
    let fixture = Fixture::new();
    fixture.write_main(test_support::SESSION_ID);
    let relative_cwd = fixture.cwd.join(RELATIVE_DIRECTORY);
    fs::create_dir(&relative_cwd).unwrap();
    let process = fixture.process(
        test_support::PROCESS_PID,
        test_support::PROCESS_START_TIME,
        &[constants::CWD_ARGUMENT, RELATIVE_DIRECTORY],
        Some(&[test_support::SESSION_ID]),
    );
    assert!(matches!(
        fixture.detector().detect(&[process]).unwrap(),
        DetectionResult::BindingChanged(binding)
            if binding.cwd == relative_cwd.canonicalize().unwrap().display().to_string()
                && binding.kind == AgentKind::Codex
    ));
}

#[test]
fn explicit_override_cannot_bypass_an_unverifiable_recorded_cwd() {
    let fixture = Fixture::new();
    fixture.write_main(test_support::SESSION_ID);
    let override_cwd = fixture.cwd.with_file_name(OVERRIDE_DIRECTORY);
    fs::create_dir(&override_cwd).unwrap();
    fs::remove_dir(&fixture.cwd).unwrap();
    let override_value = override_cwd.to_str().unwrap();
    let mut process = fixture.process(
        test_support::PROCESS_PID,
        test_support::PROCESS_START_TIME,
        &[constants::CWD_ARGUMENT, override_value],
        Some(&[test_support::SESSION_ID]),
    );
    process.cwd = Some(override_cwd.clone());

    assert_eq!(
        fixture.detector().detect(&[process]).unwrap(),
        DetectionResult::Unchanged
    );
}
