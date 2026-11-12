use super::ResumePlan;
use super::constants::{
    COMMAND_LINE_ENDING, COMMAND_PROGRAM_SLOT_COUNT, FALLBACK_SHELL, LOGIN_SHELL_ARGUMENT,
    SHELL_ENV,
};
use std::ffi::{CString, OsString};
use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

#[cfg(target_os = "macos")]
#[path = "shell_account_macos.rs"]
mod shell_account_macos;

static USER_SHELL: OnceLock<PathBuf> = OnceLock::new();

pub fn user_shell() -> PathBuf {
    USER_SHELL.get_or_init(resolve_user_shell).clone()
}

fn resolve_user_shell() -> PathBuf {
    shell_from_sources_with(
        std::env::var_os(SHELL_ENV),
        account_login_shell,
        executable_regular_file,
    )
}

fn shell_from_sources_with<A, F>(
    environment: Option<OsString>,
    mut account: A,
    mut valid: F,
) -> PathBuf
where
    A: FnMut() -> Option<OsString>,
    F: FnMut(&Path) -> bool,
{
    if let Some(candidate) = valid_candidate(environment, &mut valid) {
        return candidate;
    }
    if let Some(candidate) = valid_candidate(account(), &mut valid) {
        return candidate;
    }
    // The fixed system path makes PTY spawn fail closed if even macOS's fallback is unavailable.
    PathBuf::from(FALLBACK_SHELL)
}

fn valid_candidate<F>(value: Option<OsString>, valid: &mut F) -> Option<PathBuf>
where
    F: FnMut(&Path) -> bool,
{
    let candidate = PathBuf::from(value?);
    valid(&candidate).then_some(candidate)
}

fn executable_regular_file(path: &Path) -> bool {
    if !path.is_absolute() || !fs::metadata(path).is_ok_and(|metadata| metadata.is_file()) {
        return false;
    }
    CString::new(path.as_os_str().as_bytes())
        .is_ok_and(|path| unsafe { libc::access(path.as_ptr(), libc::X_OK) } == libc::EXIT_SUCCESS)
}

#[cfg(target_os = "macos")]
fn account_login_shell() -> Option<OsString> {
    shell_account_macos::current_user_login_shell()
}

#[cfg(not(target_os = "macos"))]
fn account_login_shell() -> Option<OsString> {
    None
}

pub fn login_arguments() -> Vec<String> {
    vec![LOGIN_SHELL_ARGUMENT.to_owned()]
}

pub fn encode_command(command: &ResumePlan) -> Vec<u8> {
    let mut words = Vec::with_capacity(command.args.len() + COMMAND_PROGRAM_SLOT_COUNT);
    words.push(shell_quote(&command.executable));
    words.extend(command.args.iter().map(|argument| shell_quote(argument)));
    let mut line = words.join(" ");
    line.push(COMMAND_LINE_ENDING);
    line.into_bytes()
}

fn shell_quote(value: &str) -> String {
    if value
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || b"_+-./:@".contains(&byte))
    {
        return value.to_owned();
    }
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/shell_tests.rs"]
mod tests;
