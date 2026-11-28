use std::os::unix::ffi::OsStrExt;
use std::process::{Command, Stdio};
use yolo_runtime::paths::SOCKET_ROOT_ENVIRONMENT;

const LINE_FEED: u8 = b'\n';
const PRINT_ENVIRONMENT_EXECUTABLE: &str = "/usr/bin/printenv";
const SOCKET_ROOT_REQUIRED: &str = "remote test socket root is not isolated";

#[test]
fn process_children_inherit_the_isolated_socket_root() {
    let expected = std::env::var_os(SOCKET_ROOT_ENVIRONMENT).expect(SOCKET_ROOT_REQUIRED);
    let expected_path = std::path::Path::new(&expected);
    assert!(expected_path.is_absolute());
    assert!(expected_path.is_dir());

    let output = Command::new(PRINT_ENVIRONMENT_EXECUTABLE)
        .arg(SOCKET_ROOT_ENVIRONMENT)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .unwrap();
    assert!(output.status.success());
    let inherited = output.stdout.strip_suffix(&[LINE_FEED]).unwrap();
    assert_eq!(inherited, expected.as_bytes());
}
