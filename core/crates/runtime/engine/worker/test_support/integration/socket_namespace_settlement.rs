use yolo_maintenance_contract::{MaintenanceScanner, ScanChunk};

const SOCKET_NAMESPACE_SETTLEMENT_TIMEOUT: Duration = Duration::from_secs(5);
const SOCKET_NAMESPACE_SETTLEMENT_DETAIL: &str =
    "isolated worker socket namespace scan did not complete before its deadline";

pub(super) fn settle_worker_test_socket_namespaces(
    socket_root: &crate::paths::IsolatedTestSocketRoot,
) -> Result<()> {
    let retained = crate::paths::directory::PrivateDirectory::open(socket_root.path())?;
    let mut scanner =
        crate::paths::test_support::scanner_for_override_root(&retained)?;
    let deadline = Instant::now() + SOCKET_NAMESPACE_SETTLEMENT_TIMEOUT;
    loop {
        if scanner.scan_chunk()? == ScanChunk::PassComplete {
            return Ok(());
        }
        anyhow::ensure!(
            Instant::now() < deadline,
            SOCKET_NAMESPACE_SETTLEMENT_DETAIL
        );
    }
}
