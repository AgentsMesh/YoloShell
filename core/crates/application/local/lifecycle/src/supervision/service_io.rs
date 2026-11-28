use anyhow::{Context, Result, ensure};
use std::{
    fs,
    io::Write,
    os::unix::fs::PermissionsExt,
    path::Path,
    process::{Command, Stdio},
};

pub(super) fn read_file(path: &Path) -> Result<String> {
    match fs::read_to_string(path) {
        Ok(text) => Ok(text),
        Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => read_privileged(path),
        Err(error) => {
            Err(error).with_context(|| format!("read effective service file {}", path.display()))
        }
    }
}

pub(super) fn file_mode(path: &Path) -> Result<fs::Permissions> {
    match fs::metadata(path) {
        Ok(metadata) => Ok(metadata.permissions()),
        Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => {
            let target = path.to_str().context("service path is not UTF-8")?;
            let output = privileged_command(["/usr/bin/stat", "-c", "%a", "--", target])?;
            let mode = std::str::from_utf8(&output.stdout)?.trim();
            let mode = u32::from_str_radix(mode, 8).context("invalid service file mode")?;
            Ok(fs::Permissions::from_mode(mode))
        }
        Err(error) => {
            Err(error).with_context(|| format!("inspect service file {}", path.display()))
        }
    }
}

pub(super) fn write_file(path: &Path, bytes: &[u8], mode: fs::Permissions) -> Result<()> {
    let temporary = path.with_extension(format!("yolo-update-{}", std::process::id()));
    match fs::write(&temporary, bytes) {
        Ok(()) => {
            let result = (|| {
                fs::set_permissions(&temporary, mode.clone())?;
                fs::rename(&temporary, path).with_context(|| {
                    format!("replace managed service configuration {}", path.display())
                })
            })();
            if result.is_err() {
                let _ = fs::remove_file(&temporary);
            }
            result
        }
        Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => {
            write_privileged(path, bytes, mode)
        }
        Err(error) => Err(error).with_context(|| {
            format!(
                "write managed service configuration {}",
                temporary.display()
            )
        }),
    }
}

fn read_privileged(path: &Path) -> Result<String> {
    let target = path.to_str().context("service path is not UTF-8")?;
    let output = privileged_command(["/bin/cat", "--", target])?;
    Ok(String::from_utf8(output.stdout)?)
}

fn write_privileged(path: &Path, bytes: &[u8], mode: fs::Permissions) -> Result<()> {
    let target = path.to_str().context("service path is not UTF-8")?;
    let mode = mode.mode() & 0o7777;
    let script = r#"
set -eu
target=$1
mode=$2
temporary=$(mktemp "$target.yolo-update.XXXXXX")
trap 'rm -f "$temporary"' EXIT
cat > "$temporary"
chmod "$mode" "$temporary"
mv -f "$temporary" "$target"
trap - EXIT
"#;
    let mut command = Command::new("sudo")
        .args([
            "-n",
            "/bin/sh",
            "-c",
            script,
            "--",
            target,
            &format!("{mode:o}"),
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .context("activate root-owned systemd configuration through sudo")?;
    let mut input = command
        .stdin
        .take()
        .context("open privileged service configuration input")?;
    input.write_all(bytes)?;
    drop(input);
    let output = command.wait_with_output()?;
    ensure!(
        output.status.success(),
        "cannot activate root-owned systemd configuration: {}",
        String::from_utf8_lossy(&output.stderr).trim()
    );
    Ok(())
}

fn privileged_command<const N: usize>(args: [&str; N]) -> Result<std::process::Output> {
    let output = Command::new("sudo")
        .arg("-n")
        .args(args)
        .output()
        .context("inspect root-owned systemd configuration through sudo")?;
    ensure!(
        output.status.success(),
        "cannot inspect root-owned systemd configuration: {}",
        String::from_utf8_lossy(&output.stderr).trim()
    );
    Ok(output)
}
