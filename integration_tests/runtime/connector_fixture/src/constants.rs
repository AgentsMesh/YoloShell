pub(crate) const PROGRAM: &str = "yolo-test-connector";
pub(crate) const SOCKET_OPTION: &str = "socket";
pub(crate) const HUB_OPTION: &str = "hub";
pub(crate) const MAX_LINE_BYTES: usize = 65_536;
pub(crate) const INPUT_BUFFER_BYTES: usize = 4_096;
pub(crate) const LINE_END: u8 = b'\n';
pub(crate) const LINE_TERMINATOR: &[u8] = b"\n";
pub(crate) const DELIMITER_BYTES: usize = 1;
pub(crate) const OPERATION_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);
pub(crate) const INPUT_IDLE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(120);
pub(crate) const INVALID_INPUT: &str =
    "Connector fixture requires a bounded Hello, Activate or Publish request line";
pub(crate) const INPUT_FAILED: &str = "Connector fixture input failed";
pub(crate) const OUTPUT_FAILED: &str =
    "Connector fixture response output failed; request may have completed";
pub(crate) const LINE_TOO_LARGE: &str = "Connector fixture line exceeds its byte limit";
pub(crate) const DEADLINE: &str = "Connector fixture deadline expired";
