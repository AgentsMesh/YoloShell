use super::*;
use std::io::Cursor;

const CAT_EXECUTABLE: &str = "/bin/cat";
const TRUE_EXECUTABLE: &str = crate::test_executables::TRUE;
const SOCKET_ROOT: &str = "/private/tmp";
const RELATIVE_ROOT: &str = "private/tmp";
const ARGUMENT: &str = "literal argument";

#[test]
fn launcher_preparation_authenticates_before_building_the_child_command() {
    let capability = ControlCapability::generate();
    let mut frame = Vec::new();
    write_capability(&mut frame, FRAME_START, &capability).unwrap();
    let command = prepare_launcher(
        &mut Cursor::new(frame),
        Path::new(TRUE_EXECUTABLE),
        Path::new(SOCKET_ROOT),
        [OsString::from(ARGUMENT)],
    )
    .unwrap();
    assert_eq!(command.get_program(), OsStr::new(TRUE_EXECUTABLE));
    assert_eq!(
        command.get_args().collect::<Vec<_>>(),
        [OsStr::new(ARGUMENT)]
    );
}

#[test]
fn launcher_preparation_rejects_invalid_inputs_before_exec() {
    let capability = ControlCapability::generate();
    let mut frame = Vec::new();
    write_capability(&mut frame, FRAME_START, &capability).unwrap();
    assert!(
        prepare_launcher(
            &mut Cursor::new(frame.clone()),
            Path::new(TRUE_EXECUTABLE),
            Path::new(RELATIVE_ROOT),
            Vec::<OsString>::new(),
        )
        .is_err()
    );
    assert!(
        prepare_launcher(
            &mut Cursor::new(frame),
            Path::new(SOCKET_ROOT),
            Path::new(SOCKET_ROOT),
            Vec::<OsString>::new(),
        )
        .is_err()
    );
    assert!(
        prepare_launcher(
            &mut Cursor::new(Vec::new()),
            Path::new(TRUE_EXECUTABLE),
            Path::new(SOCKET_ROOT),
            Vec::<OsString>::new(),
        )
        .is_err()
    );
}

#[test]
fn launcher_client_releases_a_blocked_child_or_starts_it_with_capability() {
    let mut aborted = cat_client();
    let aborted_identity = aborted.identity();
    aborted.abort().unwrap();
    assert!(!aborted_identity.matches());
    drop(aborted);

    let started = cat_client();
    let capability = ControlCapability::generate();
    let mut child = started.start(&capability).unwrap();
    assert!(child.wait().unwrap().success());
}

fn cat_client() -> LauncherClient {
    let mut command = Command::new(CAT_EXECUTABLE);
    command.stdin(Stdio::piped()).stdout(Stdio::null());
    #[cfg(target_os = "linux")]
    let input = readiness::configure(&mut command).unwrap();
    #[cfg(target_os = "macos")]
    let inheritance = {
        let discovery = ProcessDiscoveryCapability::generate().unwrap();
        let inheritance = discovery.begin_writer_inheritance().unwrap();
        inheritance
            .configure_child(&discovery, &mut command)
            .unwrap();
        inheritance
    };
    let mut child = command.spawn().unwrap();
    let identity = ProcessIdentity::capture(child.id()).unwrap();
    #[cfg(not(target_os = "linux"))]
    let input = child.stdin.take().unwrap();
    LauncherClient {
        child: Some(child),
        input: Some(input),
        identity: Some(identity),
        #[cfg(target_os = "macos")]
        inheritance: Some(inheritance),
    }
}
