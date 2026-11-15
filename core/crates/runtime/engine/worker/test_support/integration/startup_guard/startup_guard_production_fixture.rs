#[cfg(target_os = "macos")]
use super::process_descriptor;
use super::process_fixture::{
    TEST_ROOT_ROLE, TestMaster, TestRootRelease, configure_signal_resistant_root, helper_command,
    pty_size,
};
use crate::process::{PTY_SESSION_CAPABILITY_ENV, ProcessIdentity, PtySessionProof};
use crate::worker::{
    BootstrapSpawnCommand, StartupCleanupScope, StartupPtyResources, UnreleasedBootstrapGate,
    spawn_tracked_pty_root,
};
use portable_pty::native_pty_system;
use std::path::Path;
use uuid::Uuid;

pub(in crate::worker::startup::startup_guard) struct HeldStartup {
    pub(in crate::worker::startup::startup_guard) resources: StartupPtyResources<TestMaster>,
    pub(in crate::worker::startup::startup_guard) root: ProcessIdentity,
    pub(in crate::worker::startup::startup_guard) release: TestRootRelease,
    pub(in crate::worker::startup::startup_guard) cleanup_scope: StartupCleanupScope,
}

pub(in crate::worker::startup::startup_guard) struct ActiveStartup {
    pub(in crate::worker::startup::startup_guard) resources: StartupPtyResources<TestMaster>,
    pub(in crate::worker::startup::startup_guard) root: ProcessIdentity,
    pub(in crate::worker::startup::startup_guard) cleanup_scope: StartupCleanupScope,
}

impl HeldStartup {
    pub(in crate::worker::startup::startup_guard) fn release(self) -> ActiveStartup {
        let Self {
            resources,
            root,
            release,
            cleanup_scope,
        } = self;
        release.release();
        ActiveStartup {
            resources,
            root,
            cleanup_scope,
        }
    }
}

pub(in crate::worker::startup::startup_guard) fn spawn(marker: &Path) -> HeldStartup {
    spawn_with(marker, |_| {}, |_, _| {})
}

pub(in crate::worker::startup::startup_guard) fn spawn_signal_resistant(
    marker: &Path,
) -> HeldStartup {
    spawn_with(marker, configure_signal_resistant_root, |_, _| {})
}

pub(in crate::worker::startup::startup_guard) fn spawn_with_after_capture(
    marker: &Path,
    after_capture: impl FnOnce(&mut StartupPtyResources<TestMaster>, &mut StartupCleanupScope),
) -> HeldStartup {
    spawn_with(marker, |_| {}, after_capture)
}

fn spawn_with(
    marker: &Path,
    configure_root: impl FnOnce(&mut portable_pty::CommandBuilder),
    after_capture: impl FnOnce(&mut StartupPtyResources<TestMaster>, &mut StartupCleanupScope),
) -> HeldStartup {
    let mut cleanup_scope = StartupCleanupScope::new().unwrap();
    #[cfg(target_os = "macos")]
    let descriptor = process_descriptor::ParentDescriptor::stage(marker);
    let pair = native_pty_system().openpty(pty_size()).unwrap();
    let (mut proof, capability) =
        PtySessionProof::capture_fresh_bootstrap_bound(pair.master.as_ref()).unwrap();
    let bootstrap_exec = proof.take_bootstrap_exec_authority().unwrap();
    let terminal_id = Uuid::new_v4();
    let runtime_epoch = Uuid::new_v4();
    proof.bind_runtime(terminal_id, runtime_epoch).unwrap();
    #[cfg(target_os = "linux")]
    let containment = crate::worker::linux_fixture_containment::FixtureContainment::prepare(
        crate::paths::Paths::new(marker.parent().unwrap()),
        terminal_id,
        runtime_epoch,
        &proof,
    )
    .unwrap();
    let output = super::super::test_output::capture_test_output(pair.master.as_ref(), marker);
    let mut command = helper_command(marker, TEST_ROOT_ROLE);
    let release = TestRootRelease::hold(marker, &mut command);
    configure_root(&mut command);
    command.env(PTY_SESSION_CAPABILITY_ENV, capability.to_string());
    #[cfg(target_os = "macos")]
    descriptor.configure_root(&mut command);
    let command = BootstrapSpawnCommand {
        command,
        gate: UnreleasedBootstrapGate,
    };
    let child = spawn_tracked_pty_root(
        pair.slave.as_ref(),
        command,
        proof,
        output,
        &mut cleanup_scope,
    )
    .unwrap();
    let mut resources =
        StartupPtyResources::from_bootstrap_guard(pair.master, child, bootstrap_exec);
    drop(pair.slave);
    let root = resources.child().identity().unwrap();
    #[cfg(target_os = "linux")]
    containment.bind(resources.child_mut(), root).unwrap();
    after_capture(&mut resources, &mut cleanup_scope);
    #[cfg(target_os = "macos")]
    descriptor.install(resources.child_mut());
    let bootstrap_exec = resources.take_bootstrap_exec().unwrap();
    resources
        .child_mut()
        .commit_bootstrap_gate_release()
        .unwrap();
    bootstrap_exec.confirm_exec().unwrap();
    HeldStartup {
        resources,
        root,
        release,
        cleanup_scope,
    }
}
