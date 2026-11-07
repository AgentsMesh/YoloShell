#[cfg(target_os = "linux")]
const LINUX_GUARD_RELEASE_BEFORE_BINDING: &str = "Linux guard helper escaped its containment gate";

pub(in crate::worker::startup::startup_guard) fn spawn_test_guard(marker: &Path) -> TestGuard {
    #[cfg(target_os = "macos")]
    let descriptor = process_descriptor::ParentDescriptor::stage(marker);
    let pair = native_pty_system().openpty(pty_size()).unwrap();
    #[cfg(target_os = "macos")]
    let capability = Uuid::new_v4().to_string();
    #[cfg(target_os = "macos")]
    let proof =
        PtySessionProof::capture_bound_for_test(pair.master.as_ref(), capability.clone()).unwrap();
    #[cfg(target_os = "linux")]
    let (proof, capability) =
        PtySessionProof::capture_fresh_bootstrap_bound(pair.master.as_ref()).unwrap();
    #[cfg(target_os = "linux")]
    let bootstrap_exec = proof.take_bootstrap_exec_authority().unwrap();
    #[cfg(target_os = "linux")]
    let containment = crate::worker::linux_fixture_containment::FixtureContainment::prepare(
        crate::paths::Paths::new(marker.parent().unwrap()),
        Uuid::new_v4(),
        Uuid::new_v4(),
        &proof,
    )
    .unwrap();
    let output = test_output::capture_test_output(pair.master.as_ref(), marker);
    let mut command = helper_command(marker, TEST_ROOT_ROLE);
    #[cfg(target_os = "linux")]
    let release = marker.with_extension(TEST_ROOT_RELEASE_EXTENSION);
    #[cfg(target_os = "linux")]
    command.env(TEST_HELPER_RELEASE_ENV, &release);
    command.env(PTY_SESSION_CAPABILITY_ENV, capability.to_string());
    #[cfg(target_os = "macos")]
    descriptor.configure_root(&mut command);
    let child = crate::worker::spawn_waitable_pty_root(pair.slave.as_ref(), command).unwrap();
    let mut guard = StartupChildGuard::with_output(child, output);
    drop(pair.slave);
    let root = ProcessIdentity::capture(guard.process_id().unwrap()).unwrap();
    guard.track_pty(root, proof).unwrap();
    #[cfg(target_os = "linux")]
    {
        containment.bind(&mut guard, root).unwrap();
        bootstrap_exec.confirm_exec().unwrap();
        guard
            .tree
            .as_ref()
            .unwrap()
            .lock()
            .unwrap()
            .require_destructive_session()
            .unwrap();
        assert!(!marker.exists(), "{LINUX_GUARD_RELEASE_BEFORE_BINDING}");
        std::fs::write(release, []).unwrap();
    }
    #[cfg(target_os = "macos")]
    descriptor.install(&mut guard);
    (guard, pair.master, root)
}
