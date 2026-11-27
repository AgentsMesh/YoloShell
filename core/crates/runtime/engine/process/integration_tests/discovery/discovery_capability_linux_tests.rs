use super::*;
use crate::process::constants::TEST_PROCESS_DISCOVERY_CAPABILITY_VALUE as EXACT_CAPABILITY;
use std::process::{Child, Command, Stdio};

const SLEEP_EXECUTABLE: &str = "/bin/sleep";
const SLEEP_ARGUMENT: &str = "30";
const STALE_GENERATION_INCREMENT: u64 = 1;
const CAPABILITY_PREFIX_EXTENSION: &str = "-prefix";
const CAPABILITY_SUFFIX_EXTENSION: &str = "-suffix";

#[test]
fn handles_are_stable_nonzero_and_type_distinct() {
    let capability = ProcessDiscoveryCapability::generate().unwrap();
    let reconstructed = LinuxDiscoveryCapability::from_value(capability.value()).unwrap();

    assert_ne!(capability.reader_handle(), u64::default());
    assert_ne!(capability.writer_handle(), u64::default());
    assert_ne!(capability.reader_handle(), capability.writer_handle());
    assert_eq!(capability.reader_handle(), reconstructed.reader.0);
    assert_eq!(capability.writer_handle(), reconstructed.writer.0);
}

#[test]
fn writer_holder_requires_the_exact_capability_and_live_identity() {
    let capability = ProcessDiscoveryCapability::generate().unwrap();
    let foreign = ProcessDiscoveryCapability::generate().unwrap();
    let holder = HolderProcess::spawn(&capability);

    capability.verify_writer_holder(holder.identity).unwrap();
    assert!(foreign.verify_writer_holder(holder.identity).is_err());
    let stale = ProcessIdentity {
        start_time: holder
            .identity
            .start_time
            .saturating_add(STALE_GENERATION_INCREMENT),
        ..holder.identity
    };
    assert!(capability.verify_writer_holder(stale).is_err());
}

#[test]
fn writer_holder_parser_requires_an_exact_terminated_field() {
    let exact = format!("A=B\0{PROCESS_DISCOVERY_CAPABILITY_ENV}={EXACT_CAPABILITY}\0C=D\0");
    let prefix = format!(
        "{PROCESS_DISCOVERY_CAPABILITY_ENV}={EXACT_CAPABILITY}{CAPABILITY_PREFIX_EXTENSION}\0"
    );
    let suffix = format!(
        "{PROCESS_DISCOVERY_CAPABILITY_ENV}{CAPABILITY_SUFFIX_EXTENSION}={EXACT_CAPABILITY}\0"
    );
    assert!(holds_value(exact.as_bytes(), EXACT_CAPABILITY));
    assert!(!holds_value(prefix.as_bytes(), EXACT_CAPABILITY));
    assert!(!holds_value(suffix.as_bytes(), EXACT_CAPABILITY));
    assert!(!holds_value(
        format!("{PROCESS_DISCOVERY_CAPABILITY_ENV}={EXACT_CAPABILITY}").as_bytes(),
        EXACT_CAPABILITY,
    ));
}

struct HolderProcess {
    child: Child,
    identity: ProcessIdentity,
}

impl HolderProcess {
    fn spawn(capability: &ProcessDiscoveryCapability) -> Self {
        let mut command = Command::new(SLEEP_EXECUTABLE);
        command
            .arg(SLEEP_ARGUMENT)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        capability.configure_child_inheritance(&mut command);
        let child = crate::process::spawn_waitable_test_child(command);
        let identity = ProcessIdentity::capture(child.id()).unwrap();
        Self { child, identity }
    }
}

impl Drop for HolderProcess {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
