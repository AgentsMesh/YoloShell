use crate::service;
use anyhow::{Context, Result, ensure};
use std::{
    env, fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::Command,
};

#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

use crate::constants::IMAGE_NAME;
const INSTALL_ROOT_ENV: &str = "YOLO_INSTALL_ROOT";
const BIN_DIR_ENV: &str = "YOLO_BIN_DIR";

pub(crate) fn install_archive(archive: &Path) -> Result<PathBuf> {
    install_archive_at(archive, install_root()?, bin_dir()?)
}

pub(crate) fn install_archive_at(archive: &Path, root: PathBuf, bin: PathBuf) -> Result<PathBuf> {
    let work = work_dir()?;
    ensure!(!work.exists(), "temporary update directory already exists");
    fs::create_dir_all(&work)?;
    let result = install_in(&work, archive, &root, &bin);
    let _ = fs::remove_dir_all(&work);
    result
}

fn install_in(work: &Path, archive: &Path, root: &Path, bin: &Path) -> Result<PathBuf> {
    validate_entries(archive)?;
    let status = Command::new("tar")
        .args(["-xzf"])
        .arg(archive)
        .arg("-C")
        .arg(work)
        .status()
        .context("run tar for the Yolo release archive")?;
    ensure!(
        status.success(),
        "tar could not extract the Yolo release archive"
    );
    let source = work.join(IMAGE_NAME);
    let image = service::image(&source)?;
    fs::create_dir_all(root)?;
    ensure_owner(root)?;
    fs::set_permissions(root, fs::Permissions::from_mode(0o755))?;
    let image_dir = root.join(image.sha256());
    fs::create_dir_all(&image_dir)?;
    ensure_owner(&image_dir)?;
    fs::set_permissions(&image_dir, fs::Permissions::from_mode(0o700))?;
    let destination = image_dir.join(IMAGE_NAME);
    if !destination.exists() {
        let staged = image_dir.join(format!(".yolo.tmp.{}", std::process::id()));
        fs::copy(&source, &staged)?;
        fs::set_permissions(&staged, fs::Permissions::from_mode(0o755))?;
        fs::rename(&staged, &destination)?;
    }
    ensure_owner(&destination)?;
    ensure!(
        service::image(&destination)? == image,
        "installed image does not match the downloaded release"
    );
    fs::create_dir_all(bin)?;
    ensure_owner(bin)?;
    let link = bin.join(IMAGE_NAME);
    ensure!(
        !link.is_dir(),
        "cannot replace a directory at {}",
        link.display()
    );
    let staged_link = bin.join(format!(".yolo.tmp.{}", std::process::id()));
    let _ = fs::remove_file(&staged_link);
    std::os::unix::fs::symlink(&destination, &staged_link)?;
    ensure_owner(&staged_link)?;
    fs::rename(&staged_link, &link)?;
    ensure_owner(&link)?;
    Ok(link)
}

#[cfg(unix)]
fn ensure_owner(path: &Path) -> Result<()> {
    let expected = env::var("SUDO_UID")
        .ok()
        .and_then(|value| value.parse::<u32>().ok())
        .or_else(|| {
            let effective = unsafe { libc::geteuid() };
            (effective == 0).then(|| {
                home()
                    .ok()
                    .and_then(|path| fs::metadata(path).ok())
                    .map(|metadata| metadata.uid())
                    .unwrap_or(effective)
            })
        })
        .unwrap_or_else(|| unsafe { libc::geteuid() });
    let owner = fs::symlink_metadata(path)
        .with_context(|| format!("inspect install path owner {}", path.display()))?
        .uid();
    ensure!(
        owner == expected,
        "install path owner mismatch: {} belongs to uid {}, expected uid {}",
        path.display(),
        owner,
        expected
    );
    Ok(())
}

#[cfg(not(unix))]
fn ensure_owner(_path: &Path) -> Result<()> {
    Ok(())
}

fn validate_entries(archive: &Path) -> Result<()> {
    let output = Command::new("tar")
        .args(["-tzf"])
        .arg(archive)
        .output()
        .context("list the Yolo release archive")?;
    ensure!(
        output.status.success(),
        "tar could not list the release archive"
    );
    let text = String::from_utf8(output.stdout).context("release archive names are not UTF-8")?;
    let entries: Vec<&str> = text.lines().filter(|line| !line.is_empty()).collect();
    ensure!(
        entries.len() == 1 && matches!(entries[0], "yolo" | "./yolo"),
        "release archive must contain exactly one yolo executable"
    );
    Ok(())
}

fn work_dir() -> Result<PathBuf> {
    Ok(env::temp_dir().join(format!("yolo-update-{}", std::process::id())))
}

fn install_root() -> Result<PathBuf> {
    if let Some(value) = env::var_os(INSTALL_ROOT_ENV) {
        return Ok(PathBuf::from(value));
    }
    Ok(home()?.join(".local").join("lib").join("yolo"))
}

fn bin_dir() -> Result<PathBuf> {
    if let Some(value) = env::var_os(BIN_DIR_ENV) {
        return Ok(PathBuf::from(value));
    }
    Ok(home()?.join(".local").join("bin"))
}

fn home() -> Result<PathBuf> {
    #[cfg(unix)]
    if unsafe { libc::geteuid() } == 0 {
        if let Some(uid) = env::var("SUDO_UID")
            .ok()
            .and_then(|value| value.parse::<u32>().ok())
        {
            let passwd = fs::read_to_string("/etc/passwd")?;
            if let Some(home) = passwd.lines().find_map(|line| {
                let fields = line.split(':').collect::<Vec<_>>();
                (fields.len() > 5 && fields[2].parse::<u32>().ok() == Some(uid))
                    .then(|| fields[5].to_owned())
            }) {
                return Ok(home.into());
            }
            anyhow::bail!("cannot resolve the home directory for SUDO_UID={uid}");
        }
    }
    env::var_os("HOME")
        .map(PathBuf::from)
        .context("HOME is unavailable; set YOLO_INSTALL_ROOT and YOLO_BIN_DIR")
}
