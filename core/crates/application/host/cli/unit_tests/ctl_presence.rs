use super::*;
use crate::{Cli, Command};
use clap::Parser;
use std::path::Path;
use yolo_runtime::cli::constants as cli;
use yolo_runtime::host::HostConfig;
use yolo_runtime::paths::directory::PrivateDirectory;

const HOME_NAME: &str = "unstarted-home";
const PARSE_ERROR: &str = "expected a control command";

#[test]
fn eligible_read_only_queries_write_native_null_without_creating_a_home() {
    let root = tempfile::tempdir().unwrap();
    let home = root.path().join(HOME_NAME);
    for command in [
        CtlCommand::Status,
        CtlCommand::List {
            include_archived: true,
            require_complete: true,
        },
    ] {
        let options = options(&home, command);
        let absence = absence(&options).unwrap().unwrap();
        let mut bytes = Vec::new();
        write_absence(absence, &mut bytes).unwrap();
        assert_eq!(
            yolo_capnp_codec::decode::<serde_json::Value>(&bytes).unwrap(),
            serde_json::Value::Null
        );
        assert!(!home.exists());
    }
}

#[test]
fn ordinary_startup_and_shutdown_never_use_the_absence_response() {
    let root = tempfile::tempdir().unwrap();
    let home = root.path().join(HOME_NAME);
    let mut ordinary = options(&home, CtlCommand::Status);
    ordinary.existing_host = false;
    assert!(absence(&ordinary).unwrap().is_none());
    assert!(
        absence(&options(&home, CtlCommand::ShutdownIfIdle))
            .unwrap()
            .is_none()
    );
    ordinary.existing_host = true;
    ordinary.existing_socket = true;
    assert!(absence(&ordinary).unwrap().is_none());
    assert!(!home.exists());
}

#[test]
fn complete_archived_list_is_an_existing_host_read_only_query() {
    let root = tempfile::tempdir().unwrap();
    let home = root.path().join(HOME_NAME);
    let parsed = Cli::try_parse_from([
        yolo_launch_contract::EXECUTABLE_NAME,
        cli::CONTROL_COMMAND,
        cli::HOME_ARGUMENT,
        home.to_str().unwrap(),
        cli::EXISTING_HOST_ARGUMENT,
        cli::LIST_COMMAND,
        cli::INCLUDE_ARCHIVED_ARGUMENT,
        cli::REQUIRE_COMPLETE_ARGUMENT,
    ])
    .unwrap();
    let Command::Ctl(options) = parsed.command else {
        panic!("{PARSE_ERROR}")
    };
    assert!(absence(&options).unwrap().is_some());
}

#[test]
fn a_lock_holding_host_is_never_reported_absent() {
    let root = tempfile::tempdir().unwrap();
    let home = root.path().join(HOME_NAME);
    let _host = Host::open(HostConfig::new(&home)).unwrap();
    assert!(
        absence(&options(&home, CtlCommand::Status))
            .unwrap()
            .is_none()
    );
}

#[test]
fn changed_evidence_is_rejected_before_any_output() {
    let root = tempfile::tempdir().unwrap();
    let home = root.path().join(HOME_NAME);
    let proof = absence(&options(&home, CtlCommand::Status))
        .unwrap()
        .unwrap();
    PrivateDirectory::ensure(&home).unwrap();
    let mut bytes = Vec::new();
    assert!(write_absence(proof, &mut bytes).is_err());
    assert!(bytes.is_empty());
}

#[test]
fn changes_during_output_cannot_return_a_successful_absence_result() {
    let root = tempfile::tempdir().unwrap();
    let home = root.path().join(HOME_NAME);
    let proof = absence(&options(&home, CtlCommand::Status))
        .unwrap()
        .unwrap();
    assert!(write_absence(proof, CreatingWriter { home: &home }).is_err());
    assert!(home.exists());
}

#[test]
fn write_and_flush_failures_release_the_absence_reservation() {
    let root = tempfile::tempdir().unwrap();
    let home = root.path().join(HOME_NAME);
    for fail_on_flush in [false, true] {
        let proof = absence(&options(&home, CtlCommand::Status))
            .unwrap()
            .unwrap();
        assert!(write_absence(proof, FailingWriter { fail_on_flush }).is_err());
    }
    assert!(
        absence(&options(&home, CtlCommand::Status))
            .unwrap()
            .is_some()
    );
    assert!(!home.exists());
}

fn options(home: &Path, command: CtlCommand) -> CtlOptions {
    CtlOptions {
        home: Some(home.to_path_buf()),
        socket: None,
        existing_socket: false,
        existing_host: true,
        command,
    }
}

struct CreatingWriter<'a> {
    home: &'a Path,
}

impl Write for CreatingWriter<'_> {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if !self.home.exists() {
            assert!(Host::open(HostConfig::new(self.home)).is_err());
            PrivateDirectory::ensure(self.home).map_err(std::io::Error::other)?;
        }
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

struct FailingWriter {
    fail_on_flush: bool,
}

impl Write for FailingWriter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if self.fail_on_flush {
            Ok(bytes.len())
        } else {
            Err(std::io::ErrorKind::BrokenPipe.into())
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Err(std::io::ErrorKind::BrokenPipe.into())
    }
}
