use super::{KernelProcessMetadata, ProcPidInfo, metadata_with};
use anyhow::Result;

const MAX_EXACT_METADATA_ATTEMPTS: usize = 3;
const FIRST_EXACT_METADATA_ATTEMPT: usize = 1;

pub(super) fn metadata(pid: u32, query: ProcPidInfo) -> Result<Option<KernelProcessMetadata>> {
    let mut attempt = FIRST_EXACT_METADATA_ATTEMPT;
    loop {
        let result = metadata_with(pid, query);
        let permission_denied = result.as_ref().err().is_some_and(|error| {
            error
                .downcast_ref::<std::io::Error>()
                .and_then(std::io::Error::raw_os_error)
                == Some(libc::EPERM)
        });
        if !permission_denied || attempt >= MAX_EXACT_METADATA_ATTEMPTS {
            return result;
        }
        // Basic metadata can prove absence, but cannot stand in for the complete
        // identity. Requery the atomic BSD + unique-ID record after a scheduling
        // opportunity; persistent denial stays an error after the bounded attempts.
        attempt += FIRST_EXACT_METADATA_ATTEMPT;
        std::thread::yield_now();
    }
}

#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../unit_tests/macos_kernel_retry.rs"]
mod tests;
