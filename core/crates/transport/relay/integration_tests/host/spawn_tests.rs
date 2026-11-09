use super::*;
use std::path::{Path, PathBuf};

const TRUE_EXECUTABLE: &str = crate::test_executables::TRUE;
const MISSING_EXECUTABLE: &str = "/definitely/missing/yolo-host";

fn launch(executable: &str) -> HostLaunch {
    HostLaunch {
        home: PathBuf::from("/tmp/yolo-home"),
        socket: PathBuf::from("/tmp/yolo-host.sock"),
        executable: PathBuf::from(executable),
    }
}

#[test]
fn detached_host_launch_reaps_success_and_preserves_spawn_errors() {
    spawn_host(&launch(TRUE_EXECUTABLE)).unwrap();
    assert!(matches!(
        spawn_host(&launch(MISSING_EXECUTABLE)),
        Err(RelayError::HostSpawn { executable, .. })
            if executable.as_path() == Path::new(MISSING_EXECUTABLE)
    ));
}
