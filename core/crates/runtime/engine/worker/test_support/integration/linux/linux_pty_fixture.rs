#[cfg(target_os = "linux")]
const FIXTURE_GATE_FILE: &str = "pty-bootstrap.fifo";
#[cfg(target_os = "linux")]
const FIXTURE_GATE_SHELL: &str = "/bin/sh";
#[cfg(target_os = "linux")]
const FIXTURE_GATE_ARGUMENT: &str = "-c";
#[cfg(target_os = "linux")]
const FIXTURE_GATE_PROGRAM: &str = "IFS= read -r release < \"$1\" || exit; printf '%s' \"$release\" > \"$2\"; shift 2; exec \"$@\"";
#[cfg(target_os = "linux")]
const FIXTURE_GATE_ACK_FILE: &str = "pty-bootstrap.ack";
#[cfg(target_os = "linux")]
const FIXTURE_GATE_ACK_DETAIL: &str = "contained PTY fixture did not acknowledge bootstrap release";
#[cfg(target_os = "linux")]
const FIXTURE_GATE_LABEL: &str = "yolo-pty-fixture";
#[cfg(target_os = "linux")]
const FIXTURE_GATE_RELEASE: &[u8] = b"release\n";
#[cfg(target_os = "linux")]
const FIXTURE_GATE_MODE: libc::mode_t = 0o600;

impl Fixture {
    fn with_contained_command(command: CommandBuilder) -> Self {
        #[cfg(target_os = "linux")]
        return Self::with_spawn(command, linux_fixture_root);
        #[cfg(not(target_os = "linux"))]
        Self::with_command(command)
    }
}

#[cfg(target_os = "linux")]
fn linux_fixture_root(
    pair: &portable_pty::PtyPair,
    mut command: CommandBuilder,
    paths: &Paths,
    terminal_id: Uuid,
    runtime_epoch: Uuid,
    journal: Arc<Mutex<Journal>>,
) -> FixtureRoot {
    use std::os::unix::ffi::OsStrExt;
    use std::os::unix::fs::OpenOptionsExt;
    let gate = paths.session_dir(terminal_id).join(FIXTURE_GATE_FILE);
    let acknowledgement = paths.session_dir(terminal_id).join(FIXTURE_GATE_ACK_FILE);
    let gate_name = std::ffi::CString::new(gate.as_os_str().as_bytes()).unwrap();
    assert_eq!(
        unsafe { libc::mkfifo(gate_name.as_ptr(), FIXTURE_GATE_MODE) },
        0
    );
    let mut gate_file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .custom_flags(libc::O_CLOEXEC | libc::O_NONBLOCK | libc::O_NOFOLLOW)
        .open(&gate)
        .unwrap();
    let arguments = std::mem::take(command.get_argv_mut());
    command.get_argv_mut().extend([
        FIXTURE_GATE_SHELL.into(),
        FIXTURE_GATE_ARGUMENT.into(),
        FIXTURE_GATE_PROGRAM.into(),
        FIXTURE_GATE_LABEL.into(),
        gate.as_os_str().to_owned(),
        acknowledgement.as_os_str().to_owned(),
    ]);
    command.get_argv_mut().extend(arguments);
    let (mut proof, capability) =
        process::PtySessionProof::capture_fresh_bootstrap_bound(pair.master.as_ref()).unwrap();
    command.env(process::PTY_SESSION_CAPABILITY_ENV, capability.to_string());
    let exec = proof.take_bootstrap_exec_authority().unwrap();
    proof.bind_runtime(terminal_id, runtime_epoch).unwrap();
    let containment = super::linux_fixture_containment::FixtureContainment::prepare(
        paths.clone(),
        terminal_id,
        runtime_epoch,
        &proof,
    )
    .unwrap();
    let output = ExclusivePtyOutput::capture(pair.master.as_ref(), journal).unwrap();
    let mut cleanup = StartupCleanupScope::new().unwrap();
    let mut startup = spawn_tracked_pty_root(
        pair.slave.as_ref(),
        BootstrapSpawnCommand {
            command,
            gate: UnreleasedBootstrapGate,
        },
        proof,
        output,
        &mut cleanup,
    )
    .unwrap();
    let identity = startup.identity().unwrap();
    containment.bind(&mut startup, identity).unwrap();
    gate_file.write_all(FIXTURE_GATE_RELEASE).unwrap();
    startup.commit_bootstrap_gate_release().unwrap();
    let deadline = Instant::now() + crate::session_timing::WORKER_READY_TIMEOUT;
    while !acknowledgement.exists() {
        assert!(
            identity.matches() && Instant::now() < deadline,
            "{FIXTURE_GATE_ACK_DETAIL}"
        );
        std::thread::sleep(TEST_READY_POLL_INTERVAL);
    }
    exec.confirm_exec().unwrap();
    FixtureRoot {
        child: None,
        startup: Some(startup),
        identity,
    }
}
