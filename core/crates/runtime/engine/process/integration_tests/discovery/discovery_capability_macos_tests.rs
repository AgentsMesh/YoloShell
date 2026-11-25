use super::*;
use crate::process::constants::{
    PROCESS_DISCOVERY_CAPABILITY_ENV, PROCESS_DISCOVERY_MANAGER_CAPABILITY_ENV,
};
use std::ffi::{OsStr, OsString};

const TEST_EXECUTABLE: &str = "/usr/bin/true";

#[test]
fn manager_inheritance_is_not_visible_as_process_ownership() {
    let capability = ProcessDiscoveryCapability::generate().unwrap();
    let mut command = Command::new(TEST_EXECUTABLE);
    capability.configure_manager_inheritance(&mut command);

    assert_eq!(
        environment(&command, PROCESS_DISCOVERY_MANAGER_CAPABILITY_ENV),
        Some(OsString::from(capability.value()))
    );
    assert_eq!(
        environment(&command, PROCESS_DISCOVERY_CAPABILITY_ENV),
        None
    );
}

#[test]
fn writer_inheritance_remains_visible_as_process_ownership() {
    let capability = ProcessDiscoveryCapability::generate().unwrap();
    let inheritance = capability.begin_writer_inheritance().unwrap();
    let mut command = Command::new(TEST_EXECUTABLE);
    inheritance
        .configure_child(&capability, &mut command)
        .unwrap();

    assert_eq!(
        environment(&command, PROCESS_DISCOVERY_CAPABILITY_ENV),
        Some(OsString::from(capability.value()))
    );
    assert_eq!(
        environment(&command, PROCESS_DISCOVERY_MANAGER_CAPABILITY_ENV),
        None
    );
}

fn environment(command: &Command, key: &str) -> Option<OsString> {
    command
        .get_envs()
        .find(|(name, _)| name == &OsStr::new(key))
        .and_then(|(_, value)| value.map(OsStr::to_os_string))
}
