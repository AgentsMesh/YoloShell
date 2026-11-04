use super::{CapturedProcessTree, ProcessGuard};

#[cfg(target_os = "macos")]
pub(super) fn spawn(executable: &str, arguments: &[&str]) -> (ProcessGuard, CapturedProcessTree) {
    use crate::process::{
        DetachedStandardError, LiveProcessIdentity, MacosProcessCapabilityIssuer,
        PTY_SESSION_CAPABILITY_ENV, ProcessIdentity, PtySessionProof,
    };
    use std::process::{Command, Stdio};

    const DISCOVERY_LABEL: &str = "force-transition-tree";
    let (reader, writer) = MacosProcessCapabilityIssuer::create()
        .unwrap()
        .transfer_for_test()
        .unwrap();
    let mut command = Command::new(executable);
    command
        .args(arguments)
        .env(PTY_SESSION_CAPABILITY_ENV, DISCOVERY_LABEL);
    writer.configure_exec_inheritance(&mut command);
    let child = crate::process::command::spawn_detached_with_stdio(
        command,
        DetachedStandardError::Null,
        Stdio::null(),
        Stdio::null(),
        None,
    )
    .unwrap();
    let identity = ProcessIdentity::capture(child.id()).unwrap();
    let guard = ProcessGuard {
        child: Some(child),
        identity,
    };
    let live = LiveProcessIdentity::capture(identity).unwrap().unwrap();
    reader.verify_writer_holder(&live).unwrap();
    drop(writer);
    let mut proof = PtySessionProof::discovery(DISCOVERY_LABEL.to_owned()).unwrap();
    proof.install_process_capability(reader).unwrap();
    let mut tree = CapturedProcessTree::capture_pty(identity, proof).unwrap();
    tree.authorize_destructive_for_test();
    (guard, tree)
}

#[cfg(not(target_os = "macos"))]
pub(super) fn spawn(executable: &str, arguments: &[&str]) -> (ProcessGuard, CapturedProcessTree) {
    ProcessGuard::spawn(executable, arguments)
}
