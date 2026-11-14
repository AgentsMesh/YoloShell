#[cfg(target_os = "macos")]
const DESCENDANT_REPARENT_MISSING_DETAIL: &str = "PTY descendant was not reparented";

#[cfg(target_os = "macos")]
fn wait_for_descendant_reparent(
    descendant: process::ProcessIdentity,
    root_pid: u32,
    timeout: Duration,
) -> process::KernelProcessMetadataForTest {
    let deadline = Instant::now() + timeout;
    loop {
        let metadata = process::kernel_process_metadata_for_test(descendant.pid).unwrap();
        if metadata.parent_pid != root_pid {
            return metadata;
        }
        assert!(
            Instant::now() < deadline,
            "{DESCENDANT_REPARENT_MISSING_DETAIL}"
        );
        std::thread::sleep(PROCESS_DISCOVERY_POLL_INTERVAL);
    }
}
