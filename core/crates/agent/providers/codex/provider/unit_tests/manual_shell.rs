use portable_pty::{CommandBuilder, PtySize, native_pty_system};
use std::{
    fs,
    io::Write,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    thread,
    time::Instant,
};
use yolo_hook_installation::{inspect, install};

const LOGIN: &str = "-l";
const OUTPUT: &str = "YOLO_MANUAL_TEST_OUTPUT";
const SHIM: &str = r#"#!/bin/sh
if [ "$1" = NO_DAEMON_OPTION ] && [ "$2" = VERSION_OPTION ]; then
    test -z "${OLD_CODEX:-}"; exit $?
fi
printf '<%s>' "$@" >> "$YOLO_MANUAL_TEST_OUTPUT"
printf '\n' >> "$YOLO_MANUAL_TEST_OUTPUT"
exit 23
"#;

fn private_shell(shell: &Path, configured: bool, existing_function: bool) {
    let root = tempfile::tempdir().unwrap();
    let base = root.path().canonicalize().unwrap();
    fs::set_permissions(&base, fs::Permissions::from_mode(0o700)).unwrap();
    let config = base.join("codex-config");
    if configured {
        let spec = crate::install_spec();
        let preview = inspect(spec, &config).unwrap();
        install(spec, &config, &preview.revision).unwrap();
    }
    let bin = base.join("bin");
    fs::create_dir(&bin).unwrap();
    let executable = bin.join(crate::identity::KIND);
    fs::write(
        &executable,
        SHIM.replace(
            "NO_DAEMON_OPTION",
            crate::identity::managed_launch::NO_DAEMON,
        )
        .replace("VERSION_OPTION", super::constants::VERSION),
    )
    .unwrap();
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o755)).unwrap();

    let output = base.join("out");
    let is_zsh = shell.ends_with("zsh");
    let dotfiles = if is_zsh {
        base.join("dot files")
    } else {
        base.clone()
    };
    fs::create_dir_all(&dotfiles).unwrap();
    let mut startup = "export PATH=\"$YOLO_TEST_BIN:/usr/bin:/bin\"\n".to_owned();
    if existing_function {
        startup.push_str("function codex { command codex owned \"$@\"; }\n");
    }
    startup.push_str(": > \"$YOLO_MANUAL_TEST_READY\"\n");
    fs::write(
        dotfiles.join(if is_zsh { ".zshrc" } else { ".bash_profile" }),
        startup,
    )
    .unwrap();
    fs::write(dotfiles.join(".zshenv"), "export USER_ZSHENV_SEEN=once\n").unwrap();
    let mut command = CommandBuilder::new(shell);
    command.env_clear();
    command.arg(LOGIN);
    command.cwd(&base);
    command.env("HOME", &base);
    command.env("PATH", "/usr/bin:/bin");
    command.env("YOLO_TEST_BIN", &bin);
    command.env("CODEX_HOME", &config);
    command.env(OUTPUT, &output);
    let ready = base.join("ready");
    command.env("YOLO_MANUAL_TEST_READY", &ready);
    for key in [
        yolo_codex_hook_contract::TERMINAL_ENV,
        yolo_codex_hook_contract::RUNTIME_ENV,
        yolo_codex_hook_contract::SOCKET_ENV,
        yolo_codex_hook_contract::NATIVE_HOOK_EXECUTABLE_ENV,
    ] {
        command.env(key, "private-test-scope");
    }
    if is_zsh {
        command.env("ZDOTDIR", &dotfiles);
    }
    let _startup = crate::manual_shell::configure(&mut command, shell, &[LOGIN.into()]);
    let pair = native_pty_system().openpty(PtySize::default()).unwrap();
    let mut writer = pair.master.take_writer().unwrap();
    let mut reader = pair.master.try_clone_reader().unwrap();
    let transcript = base.join("shell-output");
    let mut log = fs::File::create(&transcript).unwrap();
    let drain = thread::spawn(move || std::io::copy(&mut reader, &mut log));
    let mut child = pair.slave.spawn_command(command).unwrap();
    drop(pair.slave);
    let deadline = Instant::now() + super::constants::SHELL_TIMEOUT;
    while !ready.exists() && Instant::now() < deadline {
        thread::sleep(super::constants::SHELL_POLL);
    }
    assert!(ready.exists(), "login startup did not run: {}", shell.display());
    writer.write_all(br#"codex --yolo
printf '%s\n' "$?" >> "$YOLO_MANUAL_TEST_OUTPUT"
codex --remote 'unix://path with spaces' --yolo
codex app-server
codex --disable daemon_auto_start
codex --yolo </dev/null
OLD_CODEX=1 codex --yolo
(unset YOLO_CODEX_HOOK_SOCKET; codex --yolo)
if [ -n "${ZSH_VERSION:-}" ]; then printf '%s\n' "$ZDOTDIR" "$USER_ZSHENV_SEEN" >> "$YOLO_MANUAL_TEST_OUTPUT"; fi
command rm "$YOLO_TEST_BIN/codex"
codex --yolo
printf '%s\n' "$?" >> "$YOLO_MANUAL_TEST_OUTPUT"
exit
"#).unwrap();
    let deadline = Instant::now() + super::constants::SHELL_TIMEOUT;
    while child.try_wait().unwrap().is_none() {
        if Instant::now() >= deadline {
            let _ = child.kill();
            panic!("login shell timed out: {}: {}", shell.display(), fs::read_to_string(transcript).unwrap_or_default());
        }
        thread::sleep(super::constants::SHELL_POLL);
    }
    drop(writer);
    drop(pair.master);
    let _ = drain.join().unwrap();
    let prefix = if existing_function { "<owned>" } else { "" };
    let flag = if configured && !existing_function {
        "<--no-daemon>"
    } else {
        ""
    };
    let mut expected = format!(
        "{prefix}{flag}<--yolo>\n23\n{prefix}<--remote><unix://path with spaces><--yolo>\n{prefix}<app-server>\n{prefix}<--disable><daemon_auto_start>\n{prefix}<--yolo>\n{prefix}<--yolo>\n{prefix}<--yolo>\n"
    );
    if is_zsh {
        expected.push_str(&format!("{}\nonce\n", dotfiles.display()));
    }
    expected.push_str("127\n");
    assert_eq!(
        fs::read_to_string(output).unwrap(),
        expected,
        "{}",
        shell.display()
    );
}

#[test]
fn interactive_login_shells_select_supported_local_codex_only_with_configured_hooks() {
    for shell in [PathBuf::from("/bin/bash"), PathBuf::from("/bin/zsh")] {
        if shell.is_file() {
            private_shell(&shell, true, false);
            private_shell(&shell, false, false);
            private_shell(&shell, true, true);
        }
    }
}
