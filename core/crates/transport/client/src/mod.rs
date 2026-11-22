pub(crate) use yolo_local_unix as local_unix;
pub(crate) use yolo_paths::{coordination, paths};
pub(crate) use yolo_terminal_model as model;

pub(crate) mod protocol {
    pub use yolo_protocol_validation::*;
    pub use yolo_wire::*;
}

pub(crate) mod host {
    pub use yolo_runtime_protocol::{
        CatalogueChange, HelloRequest, HelloResult, ListSessionsRequest, ListSessionsResult,
        ReconcileResult, ResumeRequest, WatchCatalogueRequest, WatchCatalogueStart,
        WorkspaceInspectRequest, WorkspaceInspectResult,
    };
}

pub(crate) mod worker {
    pub use yolo_agent_control_contract::{NativeControlObservation, NativeControlWitness};
    pub use yolo_crypto::Sha256Digest as NativeControlDigest;
    pub use yolo_lease::LeaseRequest;
    pub use yolo_runtime_protocol::{
        AttachRequest, AttachResult, FORCE_SNAPSHOT_AFTER_SEQUENCE, FencedTerminateRequest,
        InputRequest, InputResult, NativeControlLookupRequest, NativeControlOutcome,
        NativeControlResult, NativeInterruptRequest, NativeObservationRequest,
        NativeRespondRequest, NativeSubmitRequest, ResizeRequest, RuntimeRef, SignalRequest,
        SnapshotRequest, TerminalRef, WorkerAuth, attachment_next_sequence,
    };
}
#[cfg(test)]
use yolo_runtime_rpc_testing as rpc_test_support;
pub(crate) use yolo_session_timing as session_timing;
#[cfg(test)]
#[path = "../test_support/attachment.rs"]
mod attachment_test_support;

use crate::host::{ListSessionsRequest, ListSessionsResult, ReconcileResult, ResumeRequest};
use crate::model::{
    CleanupTerminationOutcome, CreateTerminal, CreateTerminalResult, CreationCleanupArchiveRequest,
    CreationCleanupDiscardRequest, CreationCleanupTerminateRequest, FinalizeCreationRequest,
    HostStatus, TerminalDiscardRequest, TerminalLifecycleSelection, TerminalRuntimeStatus,
    TerminalTerminateRequest, TerminalView, WriterCapability, WriterLease, WriterLeaseGrant,
};
use crate::protocol::{ErrorBody, ProtocolError, ResponseEnvelope, method};
use crate::worker::{
    AttachRequest, AttachResult, InputRequest, InputResult, LeaseRequest, ResizeRequest,
    RuntimeRef, SignalRequest, SnapshotRequest, TerminalRef,
};
use serde::{Serialize, de::DeserializeOwned};
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[path = "attachment.rs"]
mod attachment;
mod build;
#[path = "catalogue_watch.rs"]
mod catalogue_watch;
#[path = "completed_snapshot.rs"]
mod completed_snapshot;
#[path = "connection.rs"]
mod connection;
#[path = "constants.rs"]
mod constants;
mod creation_query;
#[path = "deadline.rs"]
mod deadline;
mod error;
#[path = "idle_shutdown.rs"]
mod idle_shutdown;
mod keep_alive;
#[path = "local_transport.rs"]
mod local_transport;
mod native_control;
#[path = "product_lifecycle.rs"]
mod product_lifecycle;
#[path = "session_listing.rs"]
mod session_listing;
#[path = "sessions.rs"]
mod sessions;
#[path = "terminal.rs"]
mod terminal;
mod workspace;

pub use attachment::Attachment;
pub use catalogue_watch::CatalogueWatch;
pub use connection::RpcConnection;
pub(crate) use constants::DEFAULT_CLIENT_OPERATION_TIMEOUT;
pub use error::ClientError;
pub use keep_alive::HostHold;

#[derive(Debug, Clone)]
pub struct Client {
    socket: PathBuf,
    expected_home: Option<PathBuf>,
    operation_timeout: std::time::Duration,
}

impl Client {
    pub fn new(socket: impl Into<PathBuf>) -> Self {
        Self {
            socket: socket.into(),
            expected_home: None,
            operation_timeout: DEFAULT_CLIENT_OPERATION_TIMEOUT,
        }
    }

    #[doc(hidden)]
    pub fn with_operation_timeout(mut self, operation_timeout: std::time::Duration) -> Self {
        self.operation_timeout = operation_timeout;
        self
    }

    pub fn managed_local(
        home: impl Into<PathBuf>,
        socket: impl Into<PathBuf>,
    ) -> Result<Self, ClientError> {
        let home = home.into();
        let expected_home =
            canonical_home(&home).map_err(|message| ClientError::InvalidLocalHome {
                home: home.clone(),
                message,
            })?;
        Ok(Self {
            socket: socket.into(),
            expected_home: Some(expected_home),
            operation_timeout: DEFAULT_CLIENT_OPERATION_TIMEOUT,
        })
    }

    pub fn socket_path(&self) -> &Path {
        &self.socket
    }

    fn operation_deadline(&self) -> deadline::ClientDeadline {
        deadline::ClientDeadline::after(self.operation_timeout)
    }
}

fn canonical_home(home: &Path) -> Result<PathBuf, String> {
    crate::paths::validate_home(home)
        .and_then(|_| crate::coordination::canonical_logical_path(home))
        .map_err(|error| error.to_string())
}

fn decode_response<T: DeserializeOwned>(response: ResponseEnvelope) -> Result<T, ClientError> {
    if !response.ok {
        return Err(response
            .error
            .unwrap_or_else(|| ErrorBody::internal("request failed"))
            .into());
    }
    Ok(response.decode_payload()?)
}

#[cfg(all(test, feature = "yolo-test-support"))]
#[path = "../unit_tests/api_matrix_fixture_values.rs"]
mod api_matrix_fixture_values;
#[cfg(all(test, feature = "yolo-test-support"))]
#[path = "../unit_tests/api_matrix_fixtures.rs"]
mod api_matrix_fixtures;
#[cfg(all(test, feature = "yolo-test-support"))]
#[path = "../unit_tests/api_matrix_support.rs"]
mod api_matrix_support;
#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../unit_tests/build.rs"]
mod build_tests;
#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../unit_tests/local_auth_tests.rs"]
mod local_auth_tests;
#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../unit_tests/product_lifecycle_api_tests.rs"]
mod product_lifecycle_api_tests;
#[cfg(test)]
#[path = "session_api_matrix_assertions.rs"]
mod session_api_matrix_assertions;
#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../unit_tests/session_api_matrix_tests.rs"]
mod session_api_matrix_tests;
#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../unit_tests/session_deadline_tests.rs"]
mod session_deadline_tests;
#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../unit_tests/session_exact_resume_tests.rs"]
mod session_exact_resume_tests;
#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../unit_tests/session_startup_deadline_tests.rs"]
mod session_startup_deadline_tests;
#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../unit_tests/shutdown_capability_tests.rs"]
mod shutdown_capability_tests;
#[cfg(test)]
#[path = "terminal_api_matrix_assertions.rs"]
mod terminal_api_matrix_assertions;
#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../unit_tests/terminal_api_matrix_tests.rs"]
mod terminal_api_matrix_tests;
#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../unit_tests/client_tests.rs"]
mod tests;
