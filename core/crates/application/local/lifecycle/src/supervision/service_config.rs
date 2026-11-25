use anyhow::{Context, Result, ensure};
use std::path::{Path, PathBuf};

use super::service_io::{file_mode, read_file, write_file};

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
    let text = read_file(path)?;
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
    write_file(path, updated.as_bytes(), file_mode(path)?)
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

pub(super) fn property<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    text.lines().find_map(|line| line.strip_prefix(key))
}
