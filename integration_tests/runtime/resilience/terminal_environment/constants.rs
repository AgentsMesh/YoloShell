pub(super) const SHELL_FILE: &str = "readline-shell";
pub(super) const SHELL_CONTENT: &str = "#!/bin/sh\nexec /bin/bash --noprofile --norc -i\n";
pub(super) const SHELL_MODE: u32 = 0o700;
pub(super) const SESSION_NAME: &str = "utf8-shell";
pub(super) const WRITER_NAME: &str = "utf8-acceptance";
pub(super) const INPUTRC_ENV: &str = "INPUTRC";
pub(super) const HISTFILE_ENV: &str = "HISTFILE";
pub(super) const PROMPT_ENV: &str = "PS1";
pub(super) const NULL_FILE: &str = "/dev/null";
pub(super) const SHELL_PROMPT: &str = "YOLO_UTF8_SHELL_READY> ";
pub(super) const LOCALE_KEYS: [&str; 3] = ["LANG", "LC_ALL", "LC_CTYPE"];
pub(super) const CODEX_SOURCE: &str = "test-codex-source";
pub(super) const CODEX_CONFIG_FILE: &str = "config.toml";
pub(super) const CODEX_CONFIG: &str = "model = 'fixture'\n";
// Split marker arguments prevent command echo from satisfying output checks.
pub(super) const READ_COMMAND: &[u8] = b"read -er -p \"$(printf '%s%s' YOLO_ READ_READY)\" yolo_ime; printf '\\n%s%s:%s\\n' YOLO_ IME_RESULT \"$yolo_ime\"\n";
pub(super) const READ_READY: &[u8] = b"YOLO_READ_READY";
pub(super) const IME_INPUT: &str = "水电费\n";
pub(super) const IME_RESULT: &str = "YOLO_IME_RESULT:水电费";
pub(super) const READLINE_NUMERIC_ARGUMENT: &[u8] = b"(arg:";
pub(super) const EXIT_COMMAND: &[u8] = b"exit\n";

#[cfg(target_os = "linux")]
pub(super) const EXPECTED_LOCALE: &str = "C.UTF-8";
#[cfg(target_os = "macos")]
pub(super) const EXPECTED_LOCALE: &str = "en_US.UTF-8";
pub(super) const LOCALE_COMMAND: &[u8] =
    b"printf '%s%s:%s:%s:%s\\n' YOLO_ LOCALE \"${LC_ALL-unset}\" \"${LC_CTYPE-unset}\" \"${LANG-unset}\"\n";
pub(super) const MANUAL_HOOK_INSTALL: &[&str] = &["hook", "install", "codex", "--yes", "--config-home"];
pub(super) const MANUAL_PROVIDER: &str = "codex";
pub(super) const MANUAL_BIN: &str = "bin";
pub(super) const MANUAL_BIN_ENV: &str = "YOLO_MANUAL_TEST_BIN";
pub(super) const MANUAL_CODEX_CONFIG: &str = "private-codex";
pub(super) const MANUAL_CODEX_TRACE: &str = "manual-argv";
pub(super) const MANUAL_TRACE_ENV: &str = "YOLO_MANUAL_TEST_TRACE";
pub(super) const MANUAL_USER_HOME: &str = "HOME";
pub(super) const MANUAL_ZDOTDIR: &str = "ZDOTDIR";
pub(super) const MANUAL_EXPORTED_FUNCTION: &str = "BASH_FUNC_codex%%";
pub(super) const MANUAL_SESSION: &str = "manual-codex";
pub(super) const MANUAL_PROVIDER_HOME: &str = "CODEX_HOME";
pub(super) const MANUAL_PATH_COMMAND: &[u8] = b"export PATH=\"$YOLO_MANUAL_TEST_BIN:$PATH\"\n";
pub(super) const MANUAL_CODEX_COMMAND: &[u8] = b"codex --yolo\n";
pub(super) const MANUAL_EXPECTED_TRACE: &str = "<--no-daemon><--yolo>\n";
pub(super) const MANUAL_POLL: std::time::Duration = std::time::Duration::from_millis(25);
#[cfg(target_os = "macos")]
pub(super) const MANUAL_TEST_SHELL: &str = "/bin/zsh";
#[cfg(not(target_os = "macos"))]
pub(super) const MANUAL_TEST_SHELL: &str = "/bin/bash";
pub(super) const MANUAL_CODEX_PROGRAM: &str = r#"#!/bin/sh
if [ "$1" = "--no-daemon" ] && [ "$2" = "--version" ]; then exit 0; fi
printf '<%s>' "$@" >> "$YOLO_MANUAL_TEST_TRACE"
printf '\n' >> "$YOLO_MANUAL_TEST_TRACE"
"#;
