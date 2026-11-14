use super::*;
use crate::paths::{IsolatedTestSocketRoot, socket_namespace::NamespaceRelease};
use std::ffi::OsStr;
use std::os::unix::ffi::OsStringExt;

const EXECUTABLE: &str = "/bin/true";
const NON_UTF8_HOME: &[u8] = b"/private/guardian-home-\xff";

#[test]
fn guardian_launch_uses_the_retained_root_without_changing_process_environment() {
    let ambient = std::env::var_os(SOCKET_ROOT_ENVIRONMENT);
    let home = tempfile::tempdir().unwrap();
    let mut root = IsolatedTestSocketRoot::create().unwrap();
    let paths = Paths::new_with_socket_root(home.path(), root.path().to_path_buf());
    let namespace = paths.retain_socket_namespace().unwrap();
    let session = Uuid::new_v4();

    let launch = command(
        Path::new(EXECUTABLE),
        &paths,
        session,
        &namespace,
        Stdio::null(),
    );

    assert_ne!(ambient.as_deref(), Some(namespace.root_path().as_os_str()));
    assert_eq!(
        socket_root(&launch),
        Some(namespace.root_path().as_os_str())
    );
    assert_eq!(
        launch.get_args().collect::<Vec<_>>(),
        arguments(paths.home(), session)
    );
    assert_eq!(std::env::var_os(SOCKET_ROOT_ENVIRONMENT), ambient);
    assert_eq!(namespace.release().unwrap(), NamespaceRelease::Removed);
    root.finalize().unwrap();
}

#[test]
fn independently_configured_guardian_commands_keep_their_own_roots() {
    let ambient = std::env::var_os(SOCKET_ROOT_ENVIRONMENT);
    let first_home = tempfile::tempdir().unwrap();
    let second_home = tempfile::tempdir().unwrap();
    let mut first_root = IsolatedTestSocketRoot::create().unwrap();
    let mut second_root = IsolatedTestSocketRoot::create().unwrap();
    let first = Paths::new_with_socket_root(first_home.path(), first_root.path().to_path_buf());
    let second = Paths::new_with_socket_root(second_home.path(), second_root.path().to_path_buf());
    let first_namespace = first.retain_socket_namespace().unwrap();
    let second_namespace = second.retain_socket_namespace().unwrap();

    let first_launch = command(
        Path::new(EXECUTABLE),
        &first,
        Uuid::new_v4(),
        &first_namespace,
        Stdio::null(),
    );
    let second_launch = command(
        Path::new(EXECUTABLE),
        &second,
        Uuid::new_v4(),
        &second_namespace,
        Stdio::null(),
    );

    assert_eq!(
        socket_root(&first_launch),
        Some(first_namespace.root_path().as_os_str())
    );
    assert_eq!(
        socket_root(&second_launch),
        Some(second_namespace.root_path().as_os_str())
    );
    assert_ne!(socket_root(&first_launch), socket_root(&second_launch));
    assert_eq!(std::env::var_os(SOCKET_ROOT_ENVIRONMENT), ambient);
    assert_eq!(
        first_namespace.release().unwrap(),
        NamespaceRelease::Removed
    );
    assert_eq!(
        second_namespace.release().unwrap(),
        NamespaceRelease::Removed
    );
    first_root.finalize().unwrap();
    second_root.finalize().unwrap();
}

#[test]
fn guardian_role_arguments_retain_exact_session_and_non_utf8_home_without_alias_selection() {
    let home = std::path::PathBuf::from(OsString::from_vec(NON_UTF8_HOME.to_vec()));
    let session = Uuid::new_v4();
    let prefix = crate::cli::ProcessRole::Host
        .arguments()
        .map(OsString::from);
    let actual = arguments(&home, session);
    assert!(actual.starts_with(&prefix));
    assert_eq!(
        &actual[prefix.len()..],
        &[
            OsString::from(crate::cli::constants::CUSTODY_GUARDIAN_COMMAND),
            crate::cli::constants::HOME_ARGUMENT.into(),
            home.as_os_str().to_owned(),
            crate::cli::constants::TERMINAL_ID_ARGUMENT.into(),
            session.to_string().into(),
        ]
    );
}

fn socket_root(command: &Command) -> Option<&OsStr> {
    command
        .get_envs()
        .find_map(|(name, value)| (name == SOCKET_ROOT_ENVIRONMENT).then_some(value).flatten())
}
