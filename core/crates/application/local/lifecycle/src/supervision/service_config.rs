use anyhow::{Context, Result, ensure};
use std::{
    fs,
    io::Write,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

pub(super) fn command_paths(command: &str) -> Result<(PathBuf, PathBuf)> {
    let command = command
        .strip_prefix("{ path=")
        .context("unknown ExecStart format")?;
    let (path, argv) = command
        .split_once(" ; argv[]=")
        .context("missing ExecStart arguments")?;
    let path = PathBuf::from(path);
    ensure!(path.is_absolute(), "service ExecStart must be absolute");
    if path.file_name().is_some_and(|name| name == "yolo") {
        return Ok((path.clone(), path));
    }
    let (argv, _) = argv
        .split_once(" ; ")
        .context("missing ExecStart argument list")?;
    let image = argv
        .strip_prefix(path.to_str().context("invalid service launcher path")?)
        .and_then(|args| args.strip_prefix(' '))
        .context("the service launcher must take the immutable yolo image as its only argument")?;
    let image = PathBuf::from(image);
    ensure!(
        image.is_absolute() && image.file_name().is_some_and(|name| name == "yolo"),
        "the service launcher must take the immutable yolo image as its only argument"
    );
    Ok((path, image))
}

pub(super) fn replace_image(path: &Path, old_image: &Path, candidate: &Path) -> Result<()> {
    let text = fs::read_to_string(path)
        .with_context(|| format!("read effective service file {}", path.display()))?;
    let old_image = old_image
        .to_str()
        .context("current service image is not UTF-8")?;
    let candidate = candidate
        .to_str()
        .context("candidate service image is not UTF-8")?;
    let updated = text.replace(old_image, candidate);
    if updated == text {
        return Ok(());
    }
    let mode = fs::metadata(path)?.permissions();
    let temporary = path.with_extension(format!("yolo-update-{}", std::process::id()));
    match fs::write(&temporary, &updated) {
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
            write_privileged(path, updated.as_bytes(), mode)
        }
        Err(error) => Err(error).with_context(|| {
            format!(
                "write managed service configuration {}",
                temporary.display()
            )
        }),
    }
}

pub(super) fn managed_service_file(path: &Path, unit: &str) -> bool {
    if !managed_root(path) {
        return false;
    }
    if path.file_name().is_some_and(|name| name == unit) {
        return true;
    }
    let dropin = format!("{unit}.d");
    path.parent()
        .and_then(Path::file_name)
        .and_then(|name| name.to_str())
        .is_some_and(|name| name == dropin.as_str())
}

fn managed_root(path: &Path) -> bool {
    [
        Path::new("/etc/systemd/system"),
        Path::new("/run/systemd/system"),
    ]
    .iter()
    .any(|root| path.starts_with(root))
        || path
            .components()
            .collect::<Vec<_>>()
            .windows(3)
            .any(|parts| {
                parts[0].as_os_str() == ".config"
                    && parts[1].as_os_str() == "systemd"
                    && parts[2].as_os_str() == "user"
            })
        || path
            .strip_prefix("/run/user")
            .ok()
            .and_then(|relative| {
                let mut components = relative.components();
                let uid = components.next()?.as_os_str().to_str()?;
                let manager = components.next()?.as_os_str();
                (uid.bytes().all(|byte| byte.is_ascii_digit()) && manager == "systemd")
                    .then_some(())
            })
            .is_some()
}

pub(super) fn managed_executable(path: &Path) -> bool {
    [Path::new("/usr/local/libexec"), Path::new("/usr/libexec")]
        .iter()
        .any(|root| path.starts_with(root))
        || path
            .components()
            .collect::<Vec<_>>()
            .windows(2)
            .any(|parts| parts[0].as_os_str() == ".local" && parts[1].as_os_str() == "bin")
}

fn write_privileged(path: &Path, bytes: &[u8], mode: std::fs::Permissions) -> Result<()> {
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
    if let Err(error) = input.write_all(bytes) {
        drop(input);
        let _ = command.kill();
        let _ = command.wait();
        return Err(error.into());
    }
    drop(input);
    let output = command.wait_with_output()?;
    ensure!(
        output.status.success(),
        "cannot activate root-owned systemd configuration: {}",
        String::from_utf8_lossy(&output.stderr).trim()
    );
    Ok(())
}

pub(super) fn property<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    text.lines().find_map(|line| line.strip_prefix(key))
}
