use super::*;

const TEST_STOPPED_OUTPUT_COMMAND: &str =
    "printf reader-backed-output; printf ready > \"$YOLO_PTY_RESUME_MARKER\"; read value";
const TEST_RESUME_MARKER_ENV: &str = "YOLO_PTY_RESUME_MARKER";
const TEST_RESUME_MARKER_FILE: &str = "resume-ready";
const TEST_SYSCALL_SUCCESS: libc::c_int = 0;
const TEST_CONTROL_BYTES: libc::ssize_t = 1;

#[test]
fn reader_backed_recovery_restarts_and_fences_ctrl_s_output() {
    let directory = tempfile::tempdir().unwrap();
    let marker = directory.path().join(TEST_RESUME_MARKER_FILE);
    let pair = open_test_pty();
    stop_output_with_ctrl_s(pair.master.as_ref());
    let lifecycle = Arc::new(ReaderLifecycle::default());
    let output = ExclusivePtyOutput::capture(pair.master.as_ref(), test_journal(directory.path()))
        .unwrap()
        .prepare_activation()
        .unwrap()
        .complete(Arc::clone(&lifecycle));
    let mut command = CommandBuilder::new(TEST_SHELL);
    command.args([TEST_COMMAND_ARGUMENT, TEST_STOPPED_OUTPUT_COMMAND]);
    command.env(TEST_RESUME_MARKER_ENV, &marker);
    let mut child = pair.slave.spawn_command(command).unwrap();
    drop(pair.slave);
    wait_for_test_marker(&marker);
    let mut recovery = output.recovery();
    assert_eq!(
        recovery.output_queue_state().unwrap(),
        OutputQueueState::Pending
    );

    let mut fence = recovery.begin_quiescence_fence().unwrap();
    assert_eq!(read_test_output(pair.master.as_ref()), TEST_OUTPUT);
    lifecycle
        .begin_output_read()
        .unwrap()
        .mark_quiescent()
        .unwrap();
    assert_eq!(
        recovery.output_queue_state().unwrap(),
        OutputQueueState::Empty
    );
    assert_eq!(
        recovery.poll_quiescence_fence(&mut fence).unwrap(),
        OutputQuiescenceState::Pending
    );
    assert!(lifecycle.output_reader_wakeup.swap(false, Ordering::AcqRel));
    lifecycle
        .begin_output_read()
        .unwrap()
        .mark_quiescent()
        .unwrap();
    assert_eq!(
        recovery.poll_quiescence_fence(&mut fence).unwrap(),
        OutputQuiescenceState::Reached
    );
    child.kill().unwrap();
    child.wait().unwrap();
}

#[test]
fn exclusive_output_fence_durably_drains_ctrl_s_queue() {
    let directory = tempfile::tempdir().unwrap();
    let marker = directory.path().join(TEST_RESUME_MARKER_FILE);
    let pair = open_test_pty();
    stop_output_with_ctrl_s(pair.master.as_ref());
    let journal = test_journal(directory.path());
    let mut output =
        ExclusivePtyOutput::capture(pair.master.as_ref(), Arc::clone(&journal)).unwrap();
    let mut command = CommandBuilder::new(TEST_SHELL);
    command.args([TEST_COMMAND_ARGUMENT, TEST_STOPPED_OUTPUT_COMMAND]);
    command.env(TEST_RESUME_MARKER_ENV, &marker);
    let mut child = pair.slave.spawn_command(command).unwrap();
    drop(pair.slave);
    wait_for_test_marker(&marker);
    assert_eq!(
        output.output_queue_state().unwrap(),
        OutputQueueState::Pending
    );

    let mut fence = ExclusivePtyOutput::begin_quiescence_fence();
    drive_exclusive_fence(&mut output, &mut fence);

    assert_eq!(
        output.output_queue_state().unwrap(),
        OutputQueueState::Empty
    );
    let transcript: Vec<u8> = crate::worker::lock_recover(&journal)
        .replay_after(u64::default(), usize::MAX)
        .unwrap()
        .into_iter()
        .flat_map(|chunk| chunk.data)
        .collect();
    assert_eq!(transcript, TEST_OUTPUT);
    child.kill().unwrap();
    child.wait().unwrap();
}

fn drive_exclusive_fence(
    output: &mut ExclusivePtyOutput,
    fence: &mut ExclusiveOutputQuiescenceFence,
) {
    let deadline = Instant::now() + TEST_OUTPUT_WAIT_TIMEOUT;
    loop {
        if output.poll_quiescence_fence(fence).unwrap() == OutputQuiescenceState::Reached {
            return;
        }
        assert!(Instant::now() < deadline);
        std::thread::sleep(TEST_OUTPUT_POLL_INTERVAL);
    }
}

fn stop_output_with_ctrl_s(master: &dyn MasterPty) {
    let fd = master.as_raw_fd().unwrap();
    let mut attributes = std::mem::MaybeUninit::<libc::termios>::uninit();
    assert_eq!(
        unsafe { libc::tcgetattr(fd, attributes.as_mut_ptr()) },
        TEST_SYSCALL_SUCCESS
    );
    let mut attributes = unsafe { attributes.assume_init() };
    attributes.c_iflag |= libc::IXON;
    assert_eq!(
        unsafe { libc::tcsetattr(fd, libc::TCSANOW, &attributes) },
        TEST_SYSCALL_SUCCESS
    );
    let stop = attributes.c_cc[libc::VSTOP];
    assert_eq!(
        unsafe {
            libc::write(
                fd,
                std::ptr::from_ref(&stop).cast(),
                TEST_CONTROL_BYTES as usize,
            )
        },
        TEST_CONTROL_BYTES
    );
}

fn wait_for_test_marker(marker: &std::path::Path) {
    let deadline = Instant::now() + TEST_OUTPUT_WAIT_TIMEOUT;
    while !marker.exists() {
        assert!(Instant::now() < deadline);
        std::thread::sleep(TEST_OUTPUT_POLL_INTERVAL);
    }
}
