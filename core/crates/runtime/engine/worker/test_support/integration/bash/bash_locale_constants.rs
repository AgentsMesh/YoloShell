use std::time::Duration;

pub(crate) const BASH: &str = "/bin/bash";
pub(crate) const BASH_ARGS: [&str; 3] = ["--noprofile", "--norc", "-i"];
pub(crate) const FIXTURE_ENV: [(&str, &str); 5] = [
    ("PATH", "/usr/bin:/bin"),
    ("INPUTRC", "/dev/null"),
    ("HISTFILE", "/dev/null"),
    ("PS1", "YOLO_LOCALE> "),
    ("PS2", "YOLO_CONTINUATION> "),
];
pub(crate) const PROMPT: &[u8] = b"YOLO_LOCALE> ";
pub(crate) const HOLDER: &str = "bash-locale-regression";
pub(crate) const READ_BYTES: usize = 4096;
pub(crate) const CAPTURE_LIMIT: usize = 64 * 1024;
pub(crate) const READ_TIMEOUT: Duration = Duration::from_secs(5);
pub(crate) const READ_POLL: Duration = Duration::from_millis(5);
pub(crate) const CLOSED_DETAIL: &str = "locale Bash PTY closed before its expected output";
pub(crate) const TIMEOUT_DETAIL: &str = "locale Bash PTY did not produce expected output";
pub(crate) const CAPTURE_DETAIL: &str = "locale Bash PTY exceeded its capture limit";
pub(crate) const LC_ALL_ENV: &str = "LC_ALL";
pub(crate) const C_LOCALE: &str = "C";
pub(crate) const DIAGNOSTIC_COMMAND: &[u8] =
    b"printf '\\nYOLO_CHARMAP='; locale charmap; bind -v; printf '\\nYOLO_DIAGNOSTIC_DONE\\n'\r";
pub(crate) const DIAGNOSTIC_END: &[u8] = b"\r\nYOLO_DIAGNOSTIC_DONE\r\n";
pub(crate) const UTF8_CHARMAP: &str = "YOLO_CHARMAP=UTF-8";
#[cfg(target_os = "linux")]
pub(crate) const ASCII_CHARMAP: &str = "YOLO_CHARMAP=ANSI_X3.4-1968";
#[cfg(target_os = "macos")]
pub(crate) const ASCII_CHARMAP: &str = "YOLO_CHARMAP=US-ASCII";
pub(crate) const CONVERT_META_OFF: &str = "set convert-meta off";
pub(crate) const CONVERT_META_ON: &str = "set convert-meta on";
pub(crate) const OUTPUT_META_ON: &str = "set output-meta on";
pub(crate) const CHINESE: &str = "水电费";
pub(crate) const ASSIGNMENT_PREFIX: &[u8] = b"value='";
pub(crate) const ASSIGNMENT_SUFFIX: &[u8] = b"'\r";
pub(crate) const VALUE_COMMAND: &[u8] = concat!(
    "printf '\\nYOLO_VALUE=%s|CHARS=%s\\n' \"$value\" \"${#value}\"; ",
    "printf '%s' \"$value\" | od -An -tx1; printf '\\nYOLO_VALUE_DONE\\n'\r"
)
.as_bytes();
pub(crate) const VALUE_END: &[u8] = b"\r\nYOLO_VALUE_DONE\r\n";
pub(crate) const EXPECTED_VALUE: &str = "YOLO_VALUE=水电费|CHARS=3";
pub(crate) const EXPECTED_UTF8_HEX: &str = "e6 b0 b4 e7 94 b5 e8 b4 b9";
pub(crate) const ARGUMENT_PROMPT: &str = "(arg: 49)";
pub(crate) const ABORT_LINE: &[u8] = b"\x07\x15";
#[cfg(target_os = "linux")]
pub(crate) const PASTE_ENABLED: &str = "set enable-bracketed-paste on";
#[cfg(target_os = "linux")]
pub(crate) const PASTE_BEGIN: &[u8] = b"\x1b[200~";
#[cfg(target_os = "linux")]
pub(crate) const PASTE_END: &[u8] = b"\x1b[201~";
