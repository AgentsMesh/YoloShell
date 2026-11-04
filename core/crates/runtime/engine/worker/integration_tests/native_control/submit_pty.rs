use super::{submit_support::*, *};
use std::io::{self, Read};
use std::time::Duration;

const READ_BYTES: usize = 256;
const POLL_INTERVAL: Duration = Duration::from_millis(5);
const SYSCALL_SUCCESS: libc::c_int = 0;
const EXISTING_DRAFT: &[u8] = b"existing native draft: ";

#[tokio::test]
async fn native_prompt_reaches_the_existing_private_pty_without_a_second_process() {
    assert_pty_input(&[]).await;
}

#[tokio::test]
async fn native_prompt_appends_without_clearing_the_existing_terminal_editor() {
    assert_pty_input(EXISTING_DRAFT).await;
}

async fn assert_pty_input(existing: &[u8]) {
    let source = prepared();
    enable_paste(&source);
    let request = submit(&source).await;
    let mut reader = {
        let master = lock_recover(&source.fixture.state.master);
        let master = master.as_ref().unwrap();
        let fd = master.as_raw_fd().unwrap();
        let mut settings = std::mem::MaybeUninit::<libc::termios>::uninit();
        // Isolated fixture PTY only: preserve literal paste delimiters and newlines.
        unsafe {
            assert_eq!(libc::tcgetattr(fd, settings.as_mut_ptr()), SYSCALL_SUCCESS);
            let mut settings = settings.assume_init();
            libc::cfmakeraw(&mut settings);
            assert_eq!(
                libc::tcsetattr(fd, libc::TCSANOW, &settings),
                SYSCALL_SUCCESS
            );
        }
        master.try_clone_reader().unwrap()
    };
    if !existing.is_empty() {
        source
            .fixture
            .state
            .write_input(
                source.fixture.controller(),
                &crate::worker::InputRequest {
                    terminal_id: request.terminal_id,
                    runtime_epoch: request.runtime_epoch,
                    generation: request.generation,
                    writer_capability: request.writer_capability.clone(),
                    input_id: uuid::Uuid::new_v4(),
                    data: existing.to_vec(),
                },
            )
            .await
            .unwrap();
    }
    let expected = [existing, PAYLOAD].concat();
    let result = source
        .fixture
        .state
        .submit_native_agent(source.fixture.controller(), &request)
        .await
        .unwrap();
    assert_eq!(result.outcome, NativeControlOutcome::Delivered);
    let output = tokio::time::timeout(ADMISSION_TIMEOUT, async {
        let mut bytes = Vec::new();
        let mut buffer = [0; READ_BYTES];
        while bytes.len() < expected.len() {
            match reader.read(&mut buffer) {
                Ok(size) if size > 0 => bytes.extend_from_slice(&buffer[..size]),
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {}
                other => panic!("private PTY stopped before complete prompt delivery: {other:?}"),
            }
            tokio::time::sleep(POLL_INTERVAL).await;
        }
        bytes
    })
    .await
    .unwrap();
    assert_eq!(output, expected);
    child_is_live(&source);
}
