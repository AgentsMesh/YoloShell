pub(super) const TERM_ENV: &str = "TERM";
pub(super) const COLORTERM_ENV: &str = "COLORTERM";
pub(super) const TERM_PROGRAM_ENV: &str = "TERM_PROGRAM";
pub(super) const LANG_ENV: &str = "LANG";
pub(super) const LC_CTYPE_ENV: &str = "LC_CTYPE";
pub(super) const LC_ALL_ENV: &str = "LC_ALL";

pub(super) const CAPABILITY_DEFAULTS: [(&str, &str); 3] = [
    (TERM_ENV, "xterm-256color"),
    (COLORTERM_ENV, "truecolor"),
    (TERM_PROGRAM_ENV, "Yolo"),
];
pub(super) const CHARACTER_LOCALE_KEYS: [&str; 3] = [LC_ALL_ENV, LC_CTYPE_ENV, LANG_ENV];

#[cfg(target_os = "linux")]
pub(super) const DEFAULT_CHARACTER_LOCALE: &str = "C.UTF-8";
#[cfg(target_os = "macos")]
pub(super) const DEFAULT_CHARACTER_LOCALE: &str = "en_US.UTF-8";
