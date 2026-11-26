//! Native Codex selection for interactive login shells with Yolo's Codex hooks installed.
use portable_pty::CommandBuilder;
use std::{fs, io, os::unix::fs::PermissionsExt, path::Path};
use tempfile::TempDir;
use yolo_hook_installation::{inspect, resolve_home, IntegrationStatus};

const BASH_FUNCTION: &str = "BASH_FUNC_codex%%";
const ZDOTDIR: &str = "ZDOTDIR";
const ORIGINAL_ZDOTDIR: &str = "YOLO_CODEX_ORIGINAL_ZDOTDIR";
const ZSHENV: &str = ".zshenv";
const LOGIN_ARGUMENT: &str = "-l";
const OWNER_DIRECTORY_MODE: u32 = 0o700;

// A later user alias or function takes precedence. The native command is looked up
// after the user's shell initialization, including PATH changes made by Node managers.
const NETWORK_FLAGS: &str = crate::identity::NETWORK_PROXY_FLAGS;
const FUNCTION: &str = r#"() {
    case $- in
        *i*) if [ "$#" = 1 ] && [ "$1" = --yolo ] && [ -t 0 ] && [ -t 1 ] &&
                [ -n "${YOLO_TERMINAL_ID:-}" ] && [ -n "${YOLO_RUNTIME_EPOCH:-}" ] &&
                [ -n "${YOLO_CODEX_HOOK_SOCKET:-}" ] && [ -n "${YOLO_HOOK_EXECUTABLE:-}" ]; then
                 if command codex FLAG DISABLE_FLAGS --version </dev/null >/dev/null 2>&1; then
                     command codex FLAG DISABLE_FLAGS "$@"
                     return $?
                 fi
             fi ;;
    esac
    command codex "$@"
}"#;
const ZSH_RESTORE: &str = r#"if (( ${+YOLO_CODEX_ORIGINAL_ZDOTDIR} )); then
    export ZDOTDIR=$YOLO_CODEX_ORIGINAL_ZDOTDIR
    unset YOLO_CODEX_ORIGINAL_ZDOTDIR
else
    unset ZDOTDIR
fi
"#;
const ZSH_USER_STARTUP: &str = r#"
if [[ -r "${ZDOTDIR-$HOME}/.zshenv" ]]; then
    builtin source "${ZDOTDIR-$HOME}/.zshenv"
fi
"#;

/// Keep the returned directory alive while the Worker owns the shell. The shell only
/// uses it to load its first startup file and immediately restores the user's ZDOTDIR.
pub fn configure(
    command: &mut CommandBuilder,
    shell: impl AsRef<Path>,
    args: &[String],
) -> Option<TempDir> {
    prepare(command, shell.as_ref(), args).unwrap_or_else(|error| {
        tracing::warn!(%error, "could not prepare interactive Codex shell integration");
        None
    })
}

fn prepare(
    command: &mut CommandBuilder,
    shell: &Path,
    args: &[String],
) -> io::Result<Option<TempDir>> {
    let shell = shell.file_name().and_then(|name| name.to_str());
    if args != [LOGIN_ARGUMENT] || !matches!(shell, Some("bash" | "zsh")) {
        return Ok(None);
    }
    let spec = crate::install_spec();
    let home = resolve_home(spec, None, |name| {
        command.get_env(name).map(ToOwned::to_owned)
    });
    if !home.is_ok_and(|home| {
        home.join(spec.file_name).is_file()
            && inspect(spec, &home).is_ok_and(|state| state.status == IntegrationStatus::Configured)
    }) {
        return Ok(None);
    }
    let function = FUNCTION
        .replace("DISABLE_FLAGS", NETWORK_FLAGS)
        .replace("FLAG", crate::identity::managed_launch::NO_DAEMON);
    if shell == Some("bash") {
        if command.get_env(BASH_FUNCTION).is_none() {
            command.env(BASH_FUNCTION, function);
        }
        return Ok(None);
    }
    let directory = tempfile::Builder::new()
        .prefix("yolo-codex-shell-")
        .permissions(fs::Permissions::from_mode(OWNER_DIRECTORY_MODE))
        .tempdir()?;
    yolo_paths::directory::PrivateDirectory::open(directory.path()).map_err(io::Error::other)?;
    let script = format!(
        "{ZSH_RESTORE}if [[ -o interactive ]] && (( ! $+functions[codex] && ! $+aliases[codex] )); then\n    function codex {}\nfi\n{ZSH_USER_STARTUP}",
        function.strip_prefix("() ").expect("shell function body")
    );
    fs::write(directory.path().join(ZSHENV), script)?;
    if let Some(original) = command.get_env(ZDOTDIR).map(ToOwned::to_owned) {
        command.env(ORIGINAL_ZDOTDIR, original);
    } else {
        command.env_remove(ORIGINAL_ZDOTDIR);
    }
    command.env(ZDOTDIR, directory.path());
    Ok(Some(directory))
}
