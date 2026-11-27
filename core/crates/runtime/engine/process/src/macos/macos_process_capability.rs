use anyhow::{Result, ensure};
use std::os::fd::{AsRawFd, OwnedFd};
use std::sync::{Arc, Mutex};

use super::macos_process_capability_fd::{
    EndpointDirection, PipeEndpointIdentity, configure_endpoint, create_private_pipe,
    set_close_on_exec,
};

const ENDPOINT_PAIR_DETAIL: &str = "process capability pipe endpoints are not reciprocal";
const ACKNOWLEDGEMENT_DETAIL: &str = "bootstrap process capability acknowledgement mismatch";
const RELEASE_READ_CONTEXT: &str = "poll process capability writer release";
const RELEASE_PROTOCOL_DETAIL: &str = "process capability writer produced unexpected data";
const RELEASE_DRAIN_BYTES: usize = 4_096;
const RELEASE_DRAIN_PASSES: usize = 16;
pub const MACOS_PROCESS_CAPABILITY_TRANSFER_MARKER: u8 = b'Y';

#[path = "macos_process_capability_discovery.rs"]
mod discovery;
#[path = "macos_process_capability_exec.rs"]
mod exec;
#[path = "macos_process_capability_monitor.rs"]
mod monitor;
#[path = "macos_process_capability_release.rs"]
mod release;
pub use monitor::MacosProcessCapabilityObservation;

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize)]
pub struct MacosProcessCapabilityIdentity {
    reader_handle: u64,
    writer_handle: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MacosProcessCapabilityRelease {
    WriterPresent,
    Released,
}

pub struct MacosProcessCapabilityIssuer {
    reader: Option<OwnedFd>,
    writer: Option<OwnedFd>,
    identity: MacosProcessCapabilityIdentity,
}

pub struct MacosProcessCapabilityManagerReader {
    descriptor: Arc<OwnedFd>,
    identity: MacosProcessCapabilityIdentity,
    monitor: Mutex<Option<monitor::MacosProcessCapabilityMonitor>>,
}

pub struct MacosProcessCapabilityChildWriter {
    descriptor: OwnedFd,
    identity: MacosProcessCapabilityIdentity,
}

impl MacosProcessCapabilityIdentity {
    pub(super) const fn from_attestation(reader_handle: u64, writer_handle: u64) -> Self {
        Self {
            reader_handle,
            writer_handle,
        }
    }

    pub fn reader_handle(self) -> u64 {
        self.reader_handle
    }

    pub fn writer_handle(self) -> u64 {
        self.writer_handle
    }
}

impl MacosProcessCapabilityIssuer {
    pub fn create() -> Result<Self> {
        let [reader, writer] = create_private_pipe()?;
        let reader_endpoint = configure_endpoint(&reader, EndpointDirection::Reader, true)?;
        let writer_endpoint = configure_endpoint(&writer, EndpointDirection::Writer, true)?;
        let identity = reciprocal_identity(reader_endpoint, writer_endpoint)?;
        Ok(Self {
            reader: Some(reader),
            writer: Some(writer),
            identity,
        })
    }

    #[cfg(any(test, feature = "adversarial-test-hooks"))]
    pub fn identity(&self) -> MacosProcessCapabilityIdentity {
        self.identity
    }

    pub fn writer_descriptor(&self) -> libc::c_int {
        self.writer.as_ref().unwrap().as_raw_fd()
    }

    pub(crate) fn into_staged(
        self,
    ) -> Result<(
        Arc<MacosProcessCapabilityManagerReader>,
        MacosProcessCapabilityChildWriter,
    )> {
        discovery::stage(self)
    }

    #[cfg(any(test, feature = "runtime-component-tests"))]
    pub fn transfer_for_test(
        self,
    ) -> Result<(
        Arc<MacosProcessCapabilityManagerReader>,
        MacosProcessCapabilityChildWriter,
    )> {
        test_support::transfer(self)
    }

    pub fn commit_acknowledgement(
        mut self,
        reader_handle: u64,
        writer_handle: u64,
    ) -> Result<Arc<MacosProcessCapabilityManagerReader>> {
        let acknowledgement = MacosProcessCapabilityIdentity {
            reader_handle,
            writer_handle,
        };
        ensure!(acknowledgement == self.identity, ACKNOWLEDGEMENT_DETAIL);
        drop(self.writer.take());
        let descriptor = self.reader.take().unwrap();
        Ok(Arc::new(MacosProcessCapabilityManagerReader {
            descriptor: Arc::new(descriptor),
            identity: self.identity,
            monitor: Mutex::new(None),
        }))
    }
}

impl MacosProcessCapabilityChildWriter {
    pub fn from_transferred(descriptor: OwnedFd) -> Result<Self> {
        let endpoint = configure_endpoint(&descriptor, EndpointDirection::Writer, true)?;
        let descriptor = super::macos_process_capability_fd::preserve_shell_writer(descriptor)?;
        Ok(Self {
            descriptor,
            identity: MacosProcessCapabilityIdentity {
                reader_handle: endpoint.peer_handle,
                writer_handle: endpoint.handle,
            },
        })
    }

    pub fn identity(&self) -> MacosProcessCapabilityIdentity {
        self.identity
    }
}

impl MacosProcessCapabilityManagerReader {
    pub(crate) fn writer_handle(&self) -> u64 {
        self.identity.writer_handle
    }
}

fn reciprocal_identity(
    reader: PipeEndpointIdentity,
    writer: PipeEndpointIdentity,
) -> Result<MacosProcessCapabilityIdentity> {
    ensure!(
        reader.handle == writer.peer_handle && writer.handle == reader.peer_handle,
        ENDPOINT_PAIR_DETAIL
    );
    Ok(MacosProcessCapabilityIdentity {
        reader_handle: reader.handle,
        writer_handle: writer.handle,
    })
}

#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/macos/macos_process_capability_monitor_exit_race_tests.rs"]
mod monitor_exit_race_tests;
#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/macos/macos_process_capability_monitor_test_support.rs"]
mod monitor_test_support;
#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/macos/macos_process_capability_monitor_tests.rs"]
mod monitor_tests;
#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/macos/macos_process_capability_queued_rights_tests.rs"]
mod queued_rights_tests;
#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/macos/macos_process_capability_reconciliation_tests.rs"]
mod reconciliation_tests;
#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/macos/macos_process_capability_secondary_tests.rs"]
mod secondary_tests;
#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/macos/macos_process_capability_secondary_transfer_process_tests.rs"]
mod secondary_transfer_process_tests;
#[cfg(all(any(test, feature = "runtime-component-tests"), feature = "yolo-test-support"))]
#[path = "../../test_support/macos/macos_process_capability.rs"]
pub(in crate::process) mod test_support;
#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/macos/macos_process_capability_tests.rs"]
mod tests;
