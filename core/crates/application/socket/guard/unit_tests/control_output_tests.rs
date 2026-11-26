use super::*;
use std::process::{Command, Stdio};

const FRAME_PAYLOAD_BYTE: u8 = b'x';
const FIRST_FRAME: &str = "first\n";
const SECOND_FRAME: &str = "second\n";
const PRINTF_EXECUTABLE: &str = "/usr/bin/printf";
const SLEEP_EXECUTABLE: &str = "/bin/sleep";
const SLEEP_SECONDS: &str = "30";
const READ_TIMEOUT: Duration = Duration::from_millis(50);

#[test]
fn complete_frame_enforces_limit_after_a_single_oversized_read() {
    let maximum = usize::try_from(MAXIMUM_CONTROL_FRAME_BYTES).unwrap();
    let mut accepted = vec![FRAME_PAYLOAD_BYTE; maximum - 1];
    accepted.push(FRAME_TERMINATOR_BYTE);
    assert_eq!(take_frame(&mut accepted).unwrap().unwrap().len(), maximum);

    let mut rejected = vec![FRAME_PAYLOAD_BYTE; maximum];
    rejected.push(FRAME_TERMINATOR_BYTE);
    assert!(take_frame(&mut rejected).is_err());
}

#[test]
fn control_output_buffers_multiple_frames_and_rejects_eof() {
    let mut child = Command::new(PRINTF_EXECUTABLE)
        .arg(format!("{FIRST_FRAME}{SECOND_FRAME}"))
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut output = ControlOutput::new(child.stdout.take().unwrap());
    // The fixed two-frame payload fits in the pipe. Reap the producer before
    // timing buffered reads so process startup is not part of the read deadline.
    assert!(child.wait().unwrap().success());
    assert_eq!(output.read_frame(READ_TIMEOUT).unwrap(), FIRST_FRAME);
    assert_eq!(output.read_frame(READ_TIMEOUT).unwrap(), SECOND_FRAME);
    assert!(output.read_frame(READ_TIMEOUT).is_err());
}

#[test]
fn control_output_times_out_when_a_live_writer_is_silent() {
    let mut child = Command::new(SLEEP_EXECUTABLE)
        .arg(SLEEP_SECONDS)
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut output = ControlOutput::new(child.stdout.take().unwrap());
    assert!(output.read_frame(READ_TIMEOUT).is_err());
    child.kill().unwrap();
    child.wait().unwrap();
}

#[test]
fn frame_extraction_rejects_invalid_utf8() {
    let mut invalid = vec![u8::MAX, FRAME_TERMINATOR_BYTE];
    assert!(take_frame(&mut invalid).is_err());
    assert!(invalid.is_empty());
}
