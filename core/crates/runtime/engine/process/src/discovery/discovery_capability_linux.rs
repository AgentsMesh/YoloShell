use super::{HEX_DIGITS_PER_BYTE, HEX_RADIX, ProcessDiscoveryCapability};
use crate::process::{
    LiveProcessIdentity, PROCESS_DISCOVERY_CAPABILITY_ENV, ProcessIdentity, procfs,
};
use anyhow::{Context, Result, ensure};
use std::io::Read;

const HANDLE_KIND_BITS: u32 = 2;
const HANDLE_PAYLOAD_BITS: u32 = u64::BITS - HANDLE_KIND_BITS;
const HANDLE_PAYLOAD_MASK: u64 = u64::MAX >> HANDLE_KIND_BITS;
const READER_HANDLE_KIND: u64 = 1;
const WRITER_HANDLE_KIND: u64 = 2;
const HANDLE_HEX_DIGITS: usize = std::mem::size_of::<u64>() * HEX_DIGITS_PER_BYTE;
const READER_HANDLE_OFFSET: usize = 0;
const WRITER_HANDLE_OFFSET: usize = READER_HANDLE_OFFSET + HANDLE_HEX_DIGITS;
const ENVIRONMENT_FIELD_SEPARATOR: u8 = 0;
const ENVIRONMENT_ASSIGNMENT_SEPARATOR: char = '=';
const ENVIRONMENT_OVERFLOW_SENTINEL_BYTES: usize = 1;
const MAXIMUM_ENVIRONMENT_BYTES: usize = 8 * 1_024 * 1_024;
const HANDLE_DERIVATION_DETAIL: &str = "derive Linux process discovery handles";
const HOLDER_GONE_DETAIL: &str = "process discovery writer holder is no longer running";
const HOLDER_MISMATCH_DETAIL: &str = "process discovery writer holder is invalid";
const ENVIRONMENT_READ_CONTEXT: &str = "read Linux process discovery writer holder";
const ENVIRONMENT_LIMIT_DETAIL: &str = "process discovery writer environment exceeds its limit";

#[derive(Clone)]
pub(super) struct LinuxDiscoveryCapability {
    reader: LinuxDiscoveryReaderHandle,
    writer: LinuxDiscoveryWriterHandle,
    origin: Option<ProcessIdentity>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct LinuxDiscoveryReaderHandle(u64);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct LinuxDiscoveryWriterHandle(u64);

impl LinuxDiscoveryCapability {
    pub(super) fn generated(value: &str) -> Result<Self> {
        let mut capability = Self::from_value(value)?;
        capability.origin = Some(ProcessIdentity::capture(std::process::id())?);
        Ok(capability)
    }
    pub(super) fn adopted(value: &str) -> Result<Self> {
        Self::from_value(value)
    }
    pub(super) fn from_value(value: &str) -> Result<Self> {
        Ok(Self {
            origin: None,
            reader: LinuxDiscoveryReaderHandle(derive_handle(
                value,
                READER_HANDLE_OFFSET,
                READER_HANDLE_KIND,
            )?),
            writer: LinuxDiscoveryWriterHandle(derive_handle(
                value,
                WRITER_HANDLE_OFFSET,
                WRITER_HANDLE_KIND,
            )?),
        })
    }

    fn verify_writer_holder(&self, identity: ProcessIdentity, value: &str) -> Result<()> {
        let live = LiveProcessIdentity::capture(identity)?.context(HOLDER_GONE_DETAIL)?;
        let environment = read_environment(live.pid())?;
        ensure!(holds_value(&environment, value), HOLDER_MISMATCH_DETAIL);
        ensure!(live.try_matches()?, HOLDER_GONE_DETAIL);
        Ok(())
    }
}

impl ProcessDiscoveryCapability {
    pub fn capture_process_tree(
        &self,
        root: ProcessIdentity,
    ) -> Result<crate::process::CapturedProcessTree> {
        self.capture_process_tree_bound(root, self.platform.origin)
    }

    #[cfg(any(test, feature = "adversarial-test-hooks"))]
    #[doc(hidden)]
    pub fn capture_process_tree_with_origin(
        &self,
        root: ProcessIdentity,
        origin: Option<ProcessIdentity>,
    ) -> Result<crate::process::CapturedProcessTree> {
        self.capture_process_tree_bound(root, origin)
    }

    fn capture_process_tree_bound(
        &self,
        root: ProcessIdentity,
        origin: Option<ProcessIdentity>,
    ) -> Result<crate::process::CapturedProcessTree> {
        let mut proof = crate::process::PtySessionProof::discovery(self.value.clone())?;
        proof.bind_linux_capability_origin(origin, root)?;
        crate::process::CapturedProcessTree::capture_pty(root, proof)
    }
    pub fn reader_handle(&self) -> u64 {
        self.platform.reader.0
    }

    pub fn writer_handle(&self) -> u64 {
        self.platform.writer.0
    }

    pub fn verify_writer_holder(&self, identity: ProcessIdentity) -> Result<()> {
        self.platform.verify_writer_holder(identity, &self.value)
    }
}

fn derive_handle(value: &str, offset: usize, kind: u64) -> Result<u64> {
    let end = offset
        .checked_add(HANDLE_HEX_DIGITS)
        .context(HANDLE_DERIVATION_DETAIL)?;
    let raw = u64::from_str_radix(
        value.get(offset..end).context(HANDLE_DERIVATION_DETAIL)?,
        u32::try_from(HEX_RADIX)?,
    )
    .context(HANDLE_DERIVATION_DETAIL)?;
    Ok((raw & HANDLE_PAYLOAD_MASK) | (kind << HANDLE_PAYLOAD_BITS))
}

fn read_environment(pid: u32) -> Result<Vec<u8>> {
    let read_limit = MAXIMUM_ENVIRONMENT_BYTES
        .checked_add(ENVIRONMENT_OVERFLOW_SENTINEL_BYTES)
        .context(ENVIRONMENT_LIMIT_DETAIL)?;
    let mut bytes = Vec::new();
    std::fs::File::open(procfs::environment_path(pid))
        .context(ENVIRONMENT_READ_CONTEXT)?
        .take(u64::try_from(read_limit)?)
        .read_to_end(&mut bytes)
        .context(ENVIRONMENT_READ_CONTEXT)?;
    ensure!(
        bytes.len() <= MAXIMUM_ENVIRONMENT_BYTES,
        ENVIRONMENT_LIMIT_DETAIL
    );
    Ok(bytes)
}

fn holds_value(environment: &[u8], value: &str) -> bool {
    if environment
        .last()
        .is_some_and(|byte| *byte != ENVIRONMENT_FIELD_SEPARATOR)
    {
        return false;
    }
    let expected =
        format!("{PROCESS_DISCOVERY_CAPABILITY_ENV}{ENVIRONMENT_ASSIGNMENT_SEPARATOR}{value}");
    environment
        .split(|byte| *byte == ENVIRONMENT_FIELD_SEPARATOR)
        .any(|field| field == expected.as_bytes())
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/discovery/discovery_capability_linux_tests.rs"]
mod tests;

#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../../unit_tests/discovery_capability_linux_origin_tests.rs"]
mod origin_tests;
