use super::*;
use std::fs;
use std::os::unix::fs::PermissionsExt;

const ACCOUNT_SHELL: &str = "/account/shell";
const ENVIRONMENT_SHELL: &str = "/environment/shell";
const EXECUTABLE_FILE_MODE: u32 = 0o700;
const MISSING_SHELL_FILE: &str = "missing-shell";
const NON_EXECUTABLE_FILE_MODE: u32 = 0o600;
const RELATIVE_SHELL: &str = "relative/shell";
const ROOT_PATH_SEPARATOR: char = '/';
const TEST_FILE_CONTENTS: &[u8] = &[];

#[test]
fn resume_arguments_are_shell_escaped() {
    let encoded = encode_command(&ResumePlan {
        executable: "agent command".into(),
        args: vec!["session'; touch /tmp/nope".into()],
        environment: Default::default(),
    });
    assert_eq!(
        String::from_utf8(encoded).unwrap(),
        "'agent command' 'session'\"'\"'; touch /tmp/nope'\n"
    );
}

#[test]
fn shell_selection_prefers_only_valid_environment_and_account_candidates() {
    assert_eq!(
        shell_from_sources_with(
            Some(ENVIRONMENT_SHELL.into()),
            || unreachable!(),
            |path| path == Path::new(ENVIRONMENT_SHELL) || path == Path::new(ACCOUNT_SHELL),
        ),
        PathBuf::from(ENVIRONMENT_SHELL)
    );
    assert_eq!(
        shell_from_sources_with(
            Some(RELATIVE_SHELL.into()),
            || Some(ACCOUNT_SHELL.into()),
            |path| path == Path::new(ACCOUNT_SHELL),
        ),
        PathBuf::from(ACCOUNT_SHELL)
    );
    for environment in [None, Some(OsString::new())] {
        assert_eq!(
            shell_from_sources_with(environment, || Some(RELATIVE_SHELL.into()), |_| false),
            PathBuf::from(FALLBACK_SHELL)
        );
    }
}

#[test]
fn shell_candidate_must_be_absolute_executable_and_regular() {
    let directory = tempfile::tempdir().unwrap();
    let executable = directory
        .path()
        .join(ENVIRONMENT_SHELL.trim_start_matches(ROOT_PATH_SEPARATOR));
    let non_executable = directory
        .path()
        .join(ACCOUNT_SHELL.trim_start_matches(ROOT_PATH_SEPARATOR));
    fs::create_dir_all(executable.parent().unwrap()).unwrap();
    fs::create_dir_all(non_executable.parent().unwrap()).unwrap();
    fs::write(&executable, TEST_FILE_CONTENTS).unwrap();
    fs::write(&non_executable, TEST_FILE_CONTENTS).unwrap();
    fs::set_permissions(
        &executable,
        fs::Permissions::from_mode(EXECUTABLE_FILE_MODE),
    )
    .unwrap();
    fs::set_permissions(
        &non_executable,
        fs::Permissions::from_mode(NON_EXECUTABLE_FILE_MODE),
    )
    .unwrap();

    assert!(executable_regular_file(&executable));
    assert!(!executable_regular_file(Path::new(RELATIVE_SHELL)));
    assert!(!executable_regular_file(directory.path()));
    assert!(!executable_regular_file(&non_executable));
    assert!(!executable_regular_file(
        &directory.path().join(MISSING_SHELL_FILE)
    ));
    assert_eq!(
        shell_from_sources_with(
            Some(non_executable.into_os_string()),
            || Some(executable.as_os_str().to_owned()),
            executable_regular_file,
        ),
        executable
    );
}
