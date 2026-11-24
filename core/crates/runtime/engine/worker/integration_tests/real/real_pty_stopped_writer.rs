const STOPPED_OUTPUT_WRITER_ENV: &str = "YOLO_REAL_PTY_STOPPED_WRITER";
const STOPPED_OUTPUT_WRITER_ROLE: &str = "progress-writer";
const STOPPED_OUTPUT_PROGRESS_ENV: &str = "YOLO_REAL_PTY_STOPPED_PROGRESS";
const STOPPED_OUTPUT_WRITER_TEST: &str =
    "worker::tests::real_pty_exit::real_pty_stopped_output_writer";
const STOPPED_OUTPUT_FRAME_PREFIX: &[u8] = b"YOLO-PTY-FRAME:";
const STOPPED_OUTPUT_FRAME_COUNTER_BYTES: usize = 16;
const STOPPED_OUTPUT_FRAME_FILL_BYTE: u8 = b'x';
const STOPPED_OUTPUT_FRAME_INCREMENT: u64 = 1;
const STOPPED_OUTPUT_PAYLOAD_BYTES: usize = 1024;
const STOPPED_OUTPUT_PROGRESS_MARKER: &[u8] = b"p";
const STOPPED_OUTPUT_PROGRESS_FILE: &str = "writer-progress";
const STOPPED_OUTPUT_READY_FILE: &str = "writer-ready";
const STOPPED_OUTPUT_RELEASE_FILE: &str = "writer-release";
const STOPPED_OUTPUT_PROGRESS_READ_DETAIL: &str = "read stopped-output writer progress";
const STOPPED_OUTPUT_PROGRESS_OPEN_DETAIL: &str = "open stopped-output writer progress";
const STOPPED_OUTPUT_PROGRESS_WRITE_DETAIL: &str = "append stopped-output writer progress";
const STOPPED_OUTPUT_PROGRESS_PATH_DETAIL: &str = "stopped-output progress path is missing";
const STOPPED_OUTPUT_READY_WRITE_DETAIL: &str = "publish stopped-output writer readiness";
const STOPPED_OUTPUT_STDOUT_WRITE_DETAIL: &str = "write stopped-output PTY payload";
const STOPPED_OUTPUT_ZERO_WRITE_DETAIL: &str = "stopped-output writer made no progress";
const STOPPED_OUTPUT_SYSCALL_FAILURE: libc::ssize_t = -1;

fn stopped_output_progress_path(directory: &Path) -> PathBuf {
    directory.join(STOPPED_OUTPUT_PROGRESS_FILE)
}

fn stopped_output_release_path(directory: &Path) -> PathBuf {
    directory.join(STOPPED_OUTPUT_RELEASE_FILE)
}

fn stopped_output_ready_path(directory: &Path) -> PathBuf {
    directory.join(STOPPED_OUTPUT_READY_FILE)
}

fn prepare_stopped_output_progress(path: &Path) {
    std::fs::File::create(path).unwrap();
}

fn release_stopped_output_writer(path: &Path) {
    std::fs::File::create(path).unwrap();
}

fn stopped_output_writer_command(progress: &Path) -> CommandBuilder {
    let mut command = CommandBuilder::new(std::env::current_exe().unwrap());
    command.args([
        BOOTSTRAP_TEST_EXACT_ARGUMENT,
        STOPPED_OUTPUT_WRITER_TEST,
        BOOTSTRAP_TEST_NOCAPTURE_ARGUMENT,
        crate::constants::TEST_HARNESS_SINGLE_THREAD_ARGUMENT,
    ]);
    command.env(STOPPED_OUTPUT_WRITER_ENV, STOPPED_OUTPUT_WRITER_ROLE);
    command.env(STOPPED_OUTPUT_PROGRESS_ENV, progress);
    command
}

#[test]
fn real_pty_stopped_output_writer() {
    if std::env::var(STOPPED_OUTPUT_WRITER_ENV).as_deref() != Ok(STOPPED_OUTPUT_WRITER_ROLE) {
        return;
    }
    let progress = std::env::var_os(STOPPED_OUTPUT_PROGRESS_ENV)
        .map(PathBuf::from)
        .expect(STOPPED_OUTPUT_PROGRESS_PATH_DETAIL);
    let directory = progress
        .parent()
        .expect(STOPPED_OUTPUT_PROGRESS_PATH_DETAIL);
    let mut progress = std::fs::OpenOptions::new()
        .append(true)
        .open(&progress)
        .expect(STOPPED_OUTPUT_PROGRESS_OPEN_DETAIL);
    publish_stopped_output_ready(&stopped_output_ready_path(directory));
    wait_for_stopped_output_release(&stopped_output_release_path(directory));
    let mut frame = u64::default();
    loop {
        let payload = stopped_output_payload(frame);
        write_stopped_output_payload(&payload).expect(STOPPED_OUTPUT_STDOUT_WRITE_DETAIL);
        progress
            .write_all(STOPPED_OUTPUT_PROGRESS_MARKER)
            .expect(STOPPED_OUTPUT_PROGRESS_WRITE_DETAIL);
        frame = frame.saturating_add(STOPPED_OUTPUT_FRAME_INCREMENT);
    }
}

fn publish_stopped_output_ready(path: &Path) {
    std::fs::File::create(path).expect(STOPPED_OUTPUT_READY_WRITE_DETAIL);
}

fn wait_for_stopped_output_ready(path: &Path) {
    wait_for_stopped_output_marker(path);
}

fn wait_for_stopped_output_release(path: &Path) {
    wait_for_stopped_output_marker(path);
}

fn wait_for_stopped_output_marker(path: &Path) {
    while !path.exists() {
        std::thread::sleep(PROCESS_DISCOVERY_POLL_INTERVAL);
    }
}

fn stopped_output_payload(frame: u64) -> [u8; STOPPED_OUTPUT_PAYLOAD_BYTES] {
    let mut payload = [STOPPED_OUTPUT_FRAME_FILL_BYTE; STOPPED_OUTPUT_PAYLOAD_BYTES];
    payload[..STOPPED_OUTPUT_FRAME_PREFIX.len()].copy_from_slice(STOPPED_OUTPUT_FRAME_PREFIX);
    let counter = format!(
        "{frame:0width$x}",
        width = STOPPED_OUTPUT_FRAME_COUNTER_BYTES
    );
    let counter_start = STOPPED_OUTPUT_FRAME_PREFIX.len();
    let counter_end = counter_start.saturating_add(STOPPED_OUTPUT_FRAME_COUNTER_BYTES);
    payload[counter_start..counter_end].copy_from_slice(counter.as_bytes());
    payload
}

fn write_stopped_output_payload(payload: &[u8]) -> std::io::Result<()> {
    let mut written = 0_usize;
    while written < payload.len() {
        let result = unsafe {
            libc::write(
                libc::STDOUT_FILENO,
                payload[written..].as_ptr().cast(),
                payload.len() - written,
            )
        };
        if result > 0 {
            written = written.saturating_add(result as usize);
            continue;
        }
        if result == STOPPED_OUTPUT_SYSCALL_FAILURE {
            let error = std::io::Error::last_os_error();
            if error.kind() == std::io::ErrorKind::Interrupted {
                continue;
            }
            return Err(error);
        }
        return Err(std::io::Error::new(
            std::io::ErrorKind::WriteZero,
            STOPPED_OUTPUT_ZERO_WRITE_DETAIL,
        ));
    }
    Ok(())
}

fn read_stopped_output_progress(path: &Path) -> Result<u64> {
    Ok(read_stopped_output_frame_count(path)?.saturating_mul(STOPPED_OUTPUT_PAYLOAD_BYTES as u64))
}

fn read_stopped_output_frame_count(path: &Path) -> Result<u64> {
    let markers = std::fs::metadata(path)
        .context(STOPPED_OUTPUT_PROGRESS_READ_DETAIL)?
        .len();
    Ok(markers)
}
