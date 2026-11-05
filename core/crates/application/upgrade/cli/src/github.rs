use crate::{
    check::Options,
    constants::*,
    github_release::{self, DEFAULT_REPOSITORY, archive_asset, checksums_asset, digest_for},
    install,
};
use anyhow::{Context, Result, bail, ensure};
use std::{env, fs, path::Path, process::Command};

const API_ROOT: &str = "https://api.github.com";
const API_ENVIRONMENT: &str = "YOLO_GITHUB_API";
const MAX_RELEASES: usize = 30;

fn latest(options: &Options) -> Result<(github_release::Release, String, String)> {
    let repository = options.repository.as_deref().unwrap_or(DEFAULT_REPOSITORY);
    validate_repository(repository)?;
    let api = env::var(API_ENVIRONMENT).unwrap_or_else(|_| API_ROOT.to_owned());
    let releases = github_release::parse(&curl_text(&releases_url(&api, repository))?)?;
    let release = github_release::latest(&releases, options.channel)?;
    let version = release.tag_name.trim_start_matches('v').to_owned();
    let name = release
        .name
        .clone()
        .unwrap_or_else(|| release.tag_name.clone());
    println!("current yolo {}", env!("CARGO_PKG_VERSION"));
    println!("github  {} ({version})", name);
    Ok((release.clone(), version, name))
}

pub(crate) fn check(options: &Options) -> Result<()> {
    let (_, version, _) = latest(options)?;
    if is_newer(&version, env!("CARGO_PKG_VERSION")) {
        println!("update available: {version}");
    } else {
        println!("yolo {version} is already current");
    }
    Ok(())
}

pub(crate) fn install(options: &Options) -> Result<std::path::PathBuf> {
    let (release, version, _) = latest(options)?;
    install_release(&release, &version)
}

/// Installs the selected release only when it is newer than this executable.
pub(crate) fn update(options: &Options) -> Result<Option<std::path::PathBuf>> {
    let (release, version, _) = latest(options)?;
    if !is_newer(&version, env!("CARGO_PKG_VERSION")) {
        println!("yolo {version} is already current");
        return Ok(None);
    }
    install_release(&release, &version).map(Some)
}

fn install_release(release: &github_release::Release, version: &str) -> Result<std::path::PathBuf> {
    let archive = archive_asset(release)?;
    let sums = checksums_asset(release)?;
    let work = work_dir()?;
    let _ = fs::remove_dir_all(&work);
    fs::create_dir_all(&work)?;
    let result = install_download(
        &work,
        &archive.name,
        &archive.browser_download_url,
        &sums.name,
        &sums.browser_download_url,
    )
    .and_then(|path| {
        println!("installed yolo {version}: {}", path.display());
        Ok(path)
    });
    let _ = fs::remove_dir_all(&work);
    result
}

fn is_newer(candidate: &str, current: &str) -> bool {
    match (components(candidate), components(current)) {
        (Some(candidate), Some(current)) => candidate > current,
        _ => candidate != current,
    }
}

fn components(version: &str) -> Option<[u64; 3]> {
    let mut parts = version.split('.');
    let mut values = [0u64; 3];
    for value in &mut values {
        *value = parts.next()?.parse().ok()?;
    }
    parts.next().is_none().then_some(values)
}

fn install_download(
    work: &Path,
    archive_name: &str,
    archive_url: &str,
    sums_name: &str,
    sums_url: &str,
) -> Result<std::path::PathBuf> {
    let archive = work.join(archive_name);
    let sums = work.join(sums_name);
    download(archive_url, &archive)?;
    download(sums_url, &sums)?;
    let expected = digest_for(&fs::read_to_string(&sums)?, archive_name)?;
    let image = crate::service::image(&archive)?;
    ensure!(
        image.sha256() == expected,
        "release archive digest does not match SHA256SUMS"
    );
    install::install_archive(&archive)
}

fn releases_url(api: &str, repository: &str) -> String {
    format!("{api}/repos/{repository}/releases?per_page={MAX_RELEASES}")
}

fn validate_repository(repository: &str) -> Result<()> {
    ensure!(
        repository.split_once('/').is_some_and(|(owner, name)| {
            !owner.is_empty()
                && !name.is_empty()
                && owner
                    .chars()
                    .chain(name.chars())
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
        }),
        "GitHub release repository must be owner/name"
    );
    Ok(())
}

fn curl_text(url: &str) -> Result<String> {
    ensure!(
        url.starts_with("https://"),
        "GitHub release URL must use HTTPS"
    );
    let mut command = curl();
    if let Ok(token) = env::var("GITHUB_TOKEN") {
        command
            .arg(CURL_HEADER)
            .arg(format!("Authorization: Bearer {token}"));
    }
    let output = command
        .args([CURL_HEADER, GITHUB_ACCEPT])
        .arg(url)
        .output()
        .context("run curl for GitHub releases")?;
    ensure!(
        output.status.success(),
        "GitHub releases request failed: {}",
        String::from_utf8_lossy(&output.stderr).trim()
    );
    String::from_utf8(output.stdout).context("GitHub releases response is not UTF-8")
}

fn download(url: &str, destination: &Path) -> Result<()> {
    ensure!(
        url.starts_with("https://"),
        "release asset URL must use HTTPS"
    );
    let output = curl()
        .args([CURL_OUTPUT])
        .arg(destination)
        .arg(url)
        .output()
        .context("download the Yolo release asset")?;
    if !output.status.success() {
        bail!(
            "release asset download failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    ensure!(
        fs::metadata(destination).is_ok_and(|metadata| metadata.len() > 0),
        "release asset download is empty"
    );
    Ok(())
}

fn curl() -> Command {
    let mut command = Command::new("curl");
    command.args([
        CURL_PROTO,
        "=https",
        CURL_PROTO_REDIRECT,
        "=https",
        CURL_TLS,
        CURL_LOCATION,
        CURL_FAIL,
        CURL_SILENT,
        CURL_SHOW_ERROR,
    ]);
    command
}

fn work_dir() -> Result<std::path::PathBuf> {
    Ok(env::temp_dir().join(format!("yolo-release-{}", std::process::id())))
}
