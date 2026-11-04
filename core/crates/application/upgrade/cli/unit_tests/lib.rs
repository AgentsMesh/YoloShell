use super::constants::{COMMAND, HELP_ARGUMENT, PROGRAM};
use super::service::ServiceUnit;
use std::{fs, os::unix::fs::PermissionsExt};
use tempfile::tempdir;

#[test]
fn service_unit_requires_a_digest_pinned_regular_image() {
    let directory = tempdir().unwrap();
    let executable = directory.path().join("yolo");
    fs::write(&executable, b"candidate").unwrap();
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o755)).unwrap();
    let image = super::service::image(&executable).unwrap();
    let unit = directory.path().join("yolo.service");
    fs::write(
        &unit,
        format!(
            "ExecStart={} internal hub serve\nX-Yolo-Image-SHA256={}\nX-Yolo-Image-Size={}\n",
            executable.display(),
            image.sha256(),
            image.size_bytes()
        ),
    )
    .unwrap();
    let parsed = ServiceUnit::read(&unit).unwrap();
    assert_eq!(parsed.image, image);
}

#[test]
fn service_unit_rejects_mutable_or_unpinned_exec_start() {
    let directory = tempdir().unwrap();
    let executable = directory.path().join("yolo");
    fs::write(&executable, b"candidate").unwrap();
    let unit = directory.path().join("yolo.service");
    fs::write(
        &unit,
        format!("ExecStart={} internal hub serve\n", executable.display()),
    )
    .unwrap();
    assert!(ServiceUnit::read(&unit).is_err());
    let link = directory.path().join("link");
    std::os::unix::fs::symlink(&executable, &link).unwrap();
    let symlink_unit = directory.path().join("symlink.service");
    fs::write(
        &symlink_unit,
        format!("ExecStart={} internal hub serve\n", link.display()),
    )
    .unwrap();
    assert!(ServiceUnit::read(&symlink_unit).is_err());
}

#[test]
fn running_the_installer_symlink_resolves_to_the_immutable_image() {
    let directory = tempdir().unwrap();
    let executable = directory.path().join("yolo");
    fs::write(&executable, b"candidate").unwrap();
    let link = directory.path().join("bin-yolo");
    std::os::unix::fs::symlink(&executable, &link).unwrap();
    assert!(super::service::image(&link).is_err());
    let resolved = fs::canonicalize(&link).unwrap();
    assert_eq!(
        super::service::image(&resolved).unwrap(),
        super::service::image(&executable).unwrap()
    );
}

#[test]
fn update_is_the_only_public_admission_command() {
    use clap::Parser;
    let args = [PROGRAM, COMMAND, HELP_ARGUMENT];
    assert_eq!(
        super::Cli::try_parse_from(args).unwrap_err().kind(),
        clap::error::ErrorKind::DisplayHelp
    );
    assert!(super::selects(&[PROGRAM.into(), COMMAND.into()]));
    assert!(!super::selects(&[PROGRAM.into(), "upgrade".into()]));
}

#[test]
fn one_failed_admission_check_cannot_be_hidden_by_later_success() {
    let executable = std::env::current_exe().unwrap();
    let image = super::service::image(&executable).unwrap();
    let mut report = super::report::CheckReport::new(image);
    report.push("first", true, "ok");
    report.push("second", false, "old image");
    report.push("third", true, "ok");
    assert!(!report.ready());
}

#[test]
fn selects_stable_and_nightly_releases_and_their_assets() {
    use super::github_release::{Channel, TARGET_TRIPLE, archive_asset, digest_for, latest, parse};
    let asset = format!("yolo-0.2.0-{TARGET_TRIPLE}.tar.gz");
    let json = format!(
        "[{{\"tag_name\":\"nightly-20261103\",\"prerelease\":true,\"draft\":false,\"published_at\":\"2026-11-03T03:00:00Z\",\"assets\":[]}},{{\"tag_name\":\"v0.2.0\",\"name\":\"Yolo 0.2.0\",\"prerelease\":false,\"draft\":false,\"published_at\":\"2026-11-02T03:00:00Z\",\"assets\":[{{\"name\":\"{asset}\",\"browser_download_url\":\"https://example.test/{asset}\"}},{{\"name\":\"SHA256SUMS\",\"browser_download_url\":\"https://example.test/SHA256SUMS\"}}]}},{{\"tag_name\":\"v0.3.0\",\"prerelease\":false,\"draft\":true,\"assets\":[]}}]"
    );
    let releases = parse(&json).unwrap();
    let stable = latest(&releases, Channel::Stable).unwrap();
    assert_eq!(stable.tag_name, "v0.2.0");
    assert_eq!(archive_asset(stable).unwrap().name, asset);
    let nightly = latest(&releases, Channel::Nightly).unwrap();
    assert_eq!(nightly.tag_name, "nightly-20261103");
    let digest = "a".repeat(64);
    assert_eq!(
        digest_for(&format!("{digest}  {asset}\n"), &asset).unwrap(),
        digest
    );
    assert!(digest_for("bad  other", &asset).is_err());
}

#[test]
fn release_flags_are_selectable_without_local_profile() {
    use super::constants::{CHANNEL_SWITCH, CHECK_RELEASE_SWITCH, COMMAND, NIGHTLY_VALUE, PROGRAM};
    use clap::Parser;
    let args = [
        PROGRAM,
        COMMAND,
        CHECK_RELEASE_SWITCH,
        CHANNEL_SWITCH,
        NIGHTLY_VALUE,
    ];
    let parsed = super::Cli::try_parse_from(args).unwrap();
    let super::Command::Update(options) = parsed.command;
    assert!(options.check_release);
    assert!(!options.install_release);
    assert_eq!(options.channel, super::github_release::Channel::Nightly);
}

#[test]
fn installs_a_release_archive_into_an_immutable_directory() {
    use std::process::Command;
    let directory = tempdir().unwrap();
    let package = directory.path().join("package");
    fs::create_dir_all(&package).unwrap();
    fs::write(package.join(super::constants::IMAGE_NAME), b"#!/bin/sh\n").unwrap();
    let archive = directory.path().join("yolo.tar.gz");
    let status = Command::new("tar")
        .args(["-czf"])
        .arg(&archive)
        .args(["-C"])
        .arg(&package)
        .arg(super::constants::IMAGE_NAME)
        .status()
        .unwrap();
    assert!(status.success());
    let root = directory.path().join("lib");
    let bin = directory.path().join("bin");
    let link = super::install::install_archive_at(&archive, root.clone(), bin.clone()).unwrap();
    let destination = fs::read_link(&link).unwrap();
    assert_eq!(
        destination,
        root.join(super::service::image(&destination).unwrap().sha256())
            .join("yolo")
    );
    assert_eq!(fs::read(&link).unwrap(), b"#!/bin/sh\n");
}
