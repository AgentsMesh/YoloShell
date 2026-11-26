//! Private filesystem acceptance driver. It never mounts or deletes anything.
use anyhow::{Result, bail};
use std::path::Path;
use yolo_release_delivery_foundation::delivery_command;
use yolo_release_delivery_workflow::{delivery, delivery_receipt};
use yolo_release_test_support as support;

#[path = "../delivery_fixture.rs"]
mod delivery_fixture;
#[path = "../delivery_recovery_support.rs"]
mod delivery_recovery_support;
mod prepare;
mod record;
mod recover;

const PREPARE: &str = "prepare";
const RECOVER: &str = "recover";
const UNSUPPORTED: &str = "reject-unsupported";
const USAGE: &str = "delivery_remount_driver prepare|recover MOUNTPOINT RECORD; \
    reject-unsupported MOUNTPOINT";

fn main() -> Result<()> {
    let arguments = std::env::args_os().skip(1).collect::<Vec<_>>();
    match arguments.as_slice() {
        [operation, mount, record] if operation == PREPARE => {
            prepare::run(Path::new(mount), Path::new(record))
        }
        [operation, mount, record] if operation == RECOVER => {
            recover::run(Path::new(mount), Path::new(record))
        }
        [operation, mount] if operation == UNSUPPORTED => {
            prepare::reject_unsupported(Path::new(mount))
        }
        _ => bail!(USAGE),
    }
}
