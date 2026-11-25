pub(super) const ENVIRONMENT_OVERRIDES: [(&str, &str); 6] = [
    ("TERM", "screen-256color"),
    ("COLORTERM", "24bit"),
    ("TERM_PROGRAM", "launch-override"),
    ("LC_ALL", "C"),
    ("LC_CTYPE", ""),
    ("LANG", ""),
];
pub(super) const ENVIRONMENT_KEYS: [&str; 6] = [
    "TERM",
    "COLORTERM",
    "TERM_PROGRAM",
    "LC_ALL",
    "LC_CTYPE",
    "LANG",
];
pub(super) const CAPABILITY_DEFAULTS: [(&str, &str); 3] = [
    ("TERM", "xterm-256color"),
    ("COLORTERM", "truecolor"),
    ("TERM_PROGRAM", "Yolo"),
];
pub(super) const LC_CTYPE_ENV: &str = "LC_CTYPE";
#[cfg(target_os = "linux")]
pub(super) const DEFAULT_CTYPE: &str = "C.UTF-8";
#[cfg(target_os = "macos")]
pub(super) const DEFAULT_CTYPE: &str = "en_US.UTF-8";
