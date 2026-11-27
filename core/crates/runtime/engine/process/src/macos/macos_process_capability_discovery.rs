use super::{
    ACKNOWLEDGEMENT_DETAIL, MacosProcessCapabilityChildWriter, MacosProcessCapabilityIdentity,
    MacosProcessCapabilityIssuer, MacosProcessCapabilityManagerReader,
};
use anyhow::{Context, Result, ensure};
use std::os::fd::{AsFd, AsRawFd, BorrowedFd, FromRawFd, OwnedFd};
use std::os::unix::process::CommandExt;
use std::process::Command;
use std::sync::{Arc, Mutex};

use super::super::LiveProcessIdentity;
use super::super::macos_process_capability_fd::{
    EndpointDirection, configure_endpoint, set_close_on_exec,
};
use crate::pty_custody::INVALID_PIPE_HANDLE;

const MINIMUM_DUPLICATED_DESCRIPTOR: libc::c_int = libc::STDERR_FILENO + 1;
const FCNTL_FAILURE: libc::c_int = -1;
const ENDPOINT_IDENTITY_DETAIL: &str = "process capability endpoint identity is invalid";
const ENDPOINT_IDENTITY_MISMATCH_DETAIL: &str =
    "process capability reader belongs to another endpoint pair";
const READER_DUPLICATE_CONTEXT: &str = "duplicate process capability reader for secondary custody";
const WRITER_HOLDER_DETAIL: &str = "exact child does not hold the process capability writer";

pub(super) fn stage(
    issuer: MacosProcessCapabilityIssuer,
) -> Result<(
    Arc<MacosProcessCapabilityManagerReader>,
    MacosProcessCapabilityChildWriter,
)> {
    let duplicated = unsafe {
        libc::fcntl(
            issuer.writer_descriptor(),
            libc::F_DUPFD_CLOEXEC,
            MINIMUM_DUPLICATED_DESCRIPTOR,
        )
    };
    ensure!(duplicated != FCNTL_FAILURE, ACKNOWLEDGEMENT_DETAIL);
    let writer = MacosProcessCapabilityChildWriter::from_transferred(unsafe {
        OwnedFd::from_raw_fd(duplicated)
    })?;
    let identity = writer.identity();
    let reader =
        issuer.commit_acknowledgement(identity.reader_handle(), identity.writer_handle())?;
    Ok((reader, writer))
}

impl MacosProcessCapabilityManagerReader {
    pub(crate) fn from_transferred(descriptor: OwnedFd) -> Result<Arc<Self>> {
        let identity = reader_identity(&descriptor)?;
        Ok(Arc::new(Self {
            descriptor: Arc::new(descriptor),
            identity,
            monitor: Mutex::new(None),
        }))
    }

    pub(crate) fn from_transferred_attested(
        descriptor: OwnedFd,
        expected: MacosProcessCapabilityIdentity,
    ) -> Result<Arc<Self>> {
        let endpoint = reader_endpoint(&descriptor)?;
        ensure!(
            attested_reader_matches(endpoint, expected),
            ENDPOINT_IDENTITY_MISMATCH_DETAIL
        );
        Ok(Arc::new(Self {
            descriptor: Arc::new(descriptor),
            identity: expected,
            monitor: Mutex::new(None),
        }))
    }

    pub(crate) fn from_transferred_exact(
        descriptor: OwnedFd,
        expected: MacosProcessCapabilityIdentity,
    ) -> Result<Arc<Self>> {
        let identity = reader_identity(&descriptor)?;
        ensure!(identity == expected, ENDPOINT_IDENTITY_MISMATCH_DETAIL);
        Ok(Arc::new(Self {
            descriptor: Arc::new(descriptor),
            identity,
            monitor: Mutex::new(None),
        }))
    }

    pub(crate) fn duplicate_for_secondary(&self) -> Result<OwnedFd> {
        let duplicated = unsafe {
            libc::fcntl(
                self.descriptor(),
                libc::F_DUPFD_CLOEXEC,
                MINIMUM_DUPLICATED_DESCRIPTOR,
            )
        };
        if duplicated == FCNTL_FAILURE {
            return Err(std::io::Error::last_os_error()).context(READER_DUPLICATE_CONTEXT);
        }
        let descriptor = unsafe { OwnedFd::from_raw_fd(duplicated) };
        let endpoint = reader_endpoint(&descriptor)?;
        ensure!(
            attested_reader_matches(endpoint, self.identity),
            ENDPOINT_IDENTITY_MISMATCH_DETAIL
        );
        Ok(descriptor)
    }

    pub(crate) fn identity(&self) -> MacosProcessCapabilityIdentity {
        self.identity
    }

    pub fn descriptor(&self) -> libc::c_int {
        self.descriptor.as_raw_fd()
    }

    pub(crate) fn as_descriptor(&self) -> BorrowedFd<'_> {
        self.descriptor.as_ref().as_fd()
    }

    pub(crate) fn configure_exec_inheritance(&self, command: &mut Command) {
        let descriptor = self.descriptor();
        unsafe {
            command.pre_exec(move || set_close_on_exec(descriptor, false));
        }
    }

    pub(crate) fn verify_writer_holder(&self, child: &LiveProcessIdentity) -> Result<()> {
        ensure!(child.try_matches()?, WRITER_HOLDER_DETAIL);
        ensure!(
            super::super::pty_session_proof::macos_descriptor_capability::exact_descriptor_holder(
                child.pid(),
                self.writer_handle()
            )?,
            WRITER_HOLDER_DETAIL
        );
        ensure!(child.try_matches()?, WRITER_HOLDER_DETAIL);
        Ok(())
    }
}

impl MacosProcessCapabilityChildWriter {
    pub fn descriptor(&self) -> libc::c_int {
        self.descriptor.as_raw_fd()
    }
}

fn reader_identity(descriptor: &OwnedFd) -> Result<MacosProcessCapabilityIdentity> {
    let endpoint = reader_endpoint(descriptor)?;
    ensure!(
        endpoint.handle != INVALID_PIPE_HANDLE
            && endpoint.peer_handle != INVALID_PIPE_HANDLE
            && endpoint.handle != endpoint.peer_handle,
        ENDPOINT_IDENTITY_DETAIL
    );
    Ok(MacosProcessCapabilityIdentity {
        reader_handle: endpoint.handle,
        writer_handle: endpoint.peer_handle,
    })
}

fn reader_endpoint(
    descriptor: &OwnedFd,
) -> Result<super::super::macos_process_capability_fd::PipeEndpointIdentity> {
    configure_endpoint(descriptor, EndpointDirection::Reader, true)
}

fn attested_reader_matches(
    endpoint: super::super::macos_process_capability_fd::PipeEndpointIdentity,
    expected: MacosProcessCapabilityIdentity,
) -> bool {
    endpoint.handle == expected.reader_handle()
        && (endpoint.peer_handle == INVALID_PIPE_HANDLE
            || endpoint.peer_handle == expected.writer_handle())
}
