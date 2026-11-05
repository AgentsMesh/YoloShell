use crate::Cli;
use crate::constants::*;
use clap::Parser;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use tempfile::tempdir;

fn parse(extra: &[&str]) -> crate::Options {
    let mut args = vec![PROGRAM, COMMAND];
    args.extend_from_slice(extra);
    let parsed = Cli::try_parse_from(args).unwrap();
    let crate::Command::Update(options) = parsed.command;
    options
}

#[test]
fn update_modes_are_explicit_and_install_activation_is_the_default() {
    let update = parse(&[]);
    assert!(!update.check && !update.admit && !update.restart && !update.install_release);
    assert!(parse(&[CHECK_SWITCH]).check);
    assert!(parse(&[CHECK_RELEASE_SWITCH]).check);
    assert!(parse(&[ADMIT_SWITCH]).admit);
    assert!(parse(&[INSTALL_RELEASE_SWITCH]).install_release);
    assert!(parse(&[NO_RESTART_SWITCH]).no_restart);
    assert!(parse(&[RESTART_SWITCH]).restart);
    assert!(parse(&[VALIDATE_STATE_FLAG]).validate_state);
}

#[test]
fn candidate_state_validation_runs_the_installed_image() {
    let directory = tempdir().unwrap();
    let executable = directory.path().join("yolo");
    let args_file = directory.path().join("args.txt");
    fs::write(
        &executable,
        format!("#!/bin/sh\nprintf '%s\\n' \"$@\" > '{}'\n", args_file.display()),
    )
    .unwrap();
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o755)).unwrap();
    let profile = directory.path().join("profile.json");
    let profile = profile.to_str().unwrap();
    let options = parse(&[VALIDATE_STATE_FLAG, LOCAL_PROFILE_SWITCH, profile]);

    crate::candidate::validate_state(&options, &executable).unwrap();
    let args: Vec<String> = fs::read_to_string(&args_file)
        .unwrap()
        .lines()
        .map(str::to_owned)
        .collect();
    assert_eq!(args.first().map(String::as_str), Some(COMMAND));
    assert_eq!(args.get(1).map(String::as_str), Some(VALIDATE_STATE_FLAG));
    assert!(args
        .windows(2)
        .any(|pair| pair[0] == LOCAL_PROFILE_SWITCH && pair[1] == profile));

    fs::write(&executable, "#!/bin/sh\nexit 3\n").unwrap();
    let error = crate::candidate::validate_state(&options, &executable).unwrap_err();
    assert!(error.to_string().contains(INSTALLED_STATE_REFUSED));
}

#[test]
fn incompatible_update_modes_are_rejected_at_parse_time() {
    for modes in [
        [CHECK_SWITCH, ADMIT_SWITCH],
        [CHECK_SWITCH, NO_RESTART_SWITCH],
        [ADMIT_SWITCH, RESTART_SWITCH],
        [INSTALL_RELEASE_SWITCH, NO_RESTART_SWITCH],
        [INSTALL_RELEASE_SWITCH, RESTART_SWITCH],
        [VALIDATE_STATE_FLAG, ADMIT_SWITCH],
        [VALIDATE_STATE_FLAG, RESTART_SWITCH],
    ] {
        let mut args = vec![PROGRAM, COMMAND];
        args.extend_from_slice(&modes);
        assert_eq!(
            Cli::try_parse_from(args).unwrap_err().kind(),
            clap::error::ErrorKind::ArgumentConflict,
            "{modes:?}"
        );
    }
}
