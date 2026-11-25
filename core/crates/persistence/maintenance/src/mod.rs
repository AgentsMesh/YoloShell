use std::path::PathBuf;
use std::time::Duration;
use yolo_maintenance_contract::ScannerSet;
#[cfg(any(test, feature = "yolo-test-support"))]
pub(crate) use yolo_maintenance_contract::{MaintenanceScanner, ScanChunk};

pub(crate) const WORKER_SHUTDOWN_GRACE_PERIOD: Duration = Duration::from_secs(5);

#[path = "control.rs"]
mod control;
#[path = "scanner_containment.rs"]
mod scanner_containment;
#[path = "scanner_loop.rs"]
mod scanner_loop;
#[cfg(any(test, feature = "yolo-test-support"))]
#[path = "../test_support/mod.rs"]
pub mod test_support;
#[path = "worker.rs"]
mod worker;

pub use worker::MaintenanceWorker;

pub(crate) struct MaintenanceGroup {
    pub(crate) owner_path: PathBuf,
    pub(crate) scanners: ScannerSet,
}

pub struct MaintenancePlan {
    pub(crate) groups: Vec<MaintenanceGroup>,
}

#[cfg(any(test, feature = "yolo-test-support"))]
impl MaintenancePlan {
    pub(crate) fn empty() -> Self {
        Self { groups: Vec::new() }
    }

    pub fn single(owner_path: PathBuf, scanners: ScannerSet) -> Self {
        Self {
            groups: vec![MaintenanceGroup {
                owner_path,
                scanners,
            }],
        }
    }
}

pub fn maintenance_plan() -> anyhow::Result<MaintenancePlan> {
    maintenance_plan_with_socket_scanner(crate::paths::SocketNamespaceScanner::new())
}

pub fn maintenance_plan_with_socket_scanner(
    mut socket_scanner: crate::paths::SocketNamespaceScanner,
) -> anyhow::Result<MaintenancePlan> {
    let uses_override = socket_scanner.uses_override();
    let socket_owner = crate::coordination::lock_path(
        crate::coordination::CoordinationScope::SocketMaintenance,
        &socket_scanner.owner_logical_path()?,
        None,
    )?;
    let mut groups = vec![MaintenanceGroup {
        owner_path: socket_owner,
        scanners: vec![Box::new(socket_scanner)],
    }];
    if !uses_override {
        groups.push(global_group());
    }
    Ok(MaintenancePlan { groups })
}

fn global_group() -> MaintenanceGroup {
    #[cfg(not(test))]
    let scanners: ScannerSet = vec![
        Box::new(crate::coordination::CoordinationScanner::new()),
        Box::new(crate::catalog::CatalogAliasScanner::new()),
    ];
    #[cfg(test)]
    let scanners = Vec::new();
    MaintenanceGroup {
        owner_path: crate::coordination::maintenance_owner_path(),
        scanners,
    }
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/isolation_tests.rs"]
mod isolation_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/reset_panic_tests.rs"]
mod reset_panic_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/shutdown_tests.rs"]
mod shutdown_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/terminal_containment_tests.rs"]
mod terminal_containment_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/tests.rs"]
mod tests;
