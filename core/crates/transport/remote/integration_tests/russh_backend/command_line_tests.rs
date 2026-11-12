use super::*;
use crate::remote::constants::{BRIDGE_COMMAND, HOME_ARGUMENT, STDIO_ARGUMENT};
use crate::remote::{CommandPolicy, CommandRequest};
use std::ffi::OsString;
use std::os::unix::ffi::OsStringExt;

#[test]
fn quotes_shell_metacharacters_and_single_quotes() {
    let policy = CommandPolicy::new(["/usr/bin/printf".into()]).unwrap();
    let command = policy
        .authorize(
            CommandRequest::new(
                "/usr/bin/printf",
                vec!["%s".into(), "a'; touch /tmp/bad; '".into()],
            )
            .with_cwd("/tmp/a b"),
        )
        .unwrap();
    assert_eq!(
        command_line(&command).unwrap(),
        "cd -- '/tmp/a b' && exec '/usr/bin/printf' '%s' 'a'\"'\"'; touch /tmp/bad; '\"'\"''"
    );
}

#[test]
fn runtime_command_has_no_unquoted_paths() {
    let runtime = RemoteRuntimeSpec::new("/opt/Yolo bin/yolo", "/tmp/Yolo state").unwrap();
    assert_eq!(
        runtime_command(&runtime).unwrap(),
        [
            "'/opt/Yolo bin/yolo'",
            &format!("'{}'", yolo_launch_contract::INTERNAL_COMMAND),
            &format!("'{}'", yolo_launch_contract::ProcessRole::Host.name()),
            &format!("'{BRIDGE_COMMAND}'"),
            &format!("'{STDIO_ARGUMENT}'"),
            &format!("'{HOME_ARGUMENT}'"),
            "'/tmp/Yolo state'",
        ]
        .join(SHELL_SEPARATOR)
    );
}

#[test]
fn invalid_utf8_and_nul_are_rejected_before_exec() {
    let bad_path = std::path::PathBuf::from(OsString::from_vec(vec![b'/', 0xff]));
    assert!(RemoteRuntimeSpec::new(bad_path, "/tmp/yolo").is_err());

    let request = CommandRequest::new("/usr/bin/printf", vec!["bad\0arg".into()]);
    let command = AuthorizedCommand(request);
    assert!(command_line(&command).is_err());

    let request = CommandRequest::new("/usr/bin/printf", vec![]).with_cwd(
        std::path::PathBuf::from(OsString::from_vec(vec![b'/', 0xff])),
    );
    let command = AuthorizedCommand(request);
    assert!(command_line(&command).is_err());
}
