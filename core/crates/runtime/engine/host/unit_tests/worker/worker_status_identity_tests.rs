use crate::host::tests::*;
use super::*;

const STATUS_IDENTITY_SOCKET_NAME: &str = "status-identity.sock";
const STATUS_IDENTITY_PID_OFFSET: u32 = 1;
const STATUS_IDENTITY_START_OFFSET: u64 = 1;

#[derive(Clone, Copy)]
enum StatusIdentityMismatch {
    RuntimeEpoch,
    Worker,
    WorkerPid,
    WorkerStart,
    ChildPid,
    ChildStart,
}

#[tokio::test]
async fn worker_status_identity_is_bound_to_authenticated_hello() {
    for mismatch in [
        StatusIdentityMismatch::RuntimeEpoch,
        StatusIdentityMismatch::Worker,
        StatusIdentityMismatch::WorkerPid,
        StatusIdentityMismatch::WorkerStart,
        StatusIdentityMismatch::ChildPid,
        StatusIdentityMismatch::ChildStart,
    ] {
        assert_status_identity_mismatch(mismatch).await;
    }
}

async fn assert_status_identity_mismatch(mismatch: StatusIdentityMismatch) {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let socket = directory.path().join(STATUS_IDENTITY_SOCKET_NAME);
    let listener = UnixListener::bind(&socket).unwrap();
    let record = host_runtime_record(&host, Uuid::new_v4(), socket);
    let mut status = status_view(&record, Some(Vec::new()));
    apply_status_identity_mismatch(&mut status, mismatch);
    let server_record = record.clone();
    let server = tokio::spawn(async move {
        serve_capability_status(listener, server_record, status).await;
    });

    let error = host.worker_status(&record).await.unwrap_err();
    assert!(matches!(error, WorkerRpcError::IdentityMismatch(_)));
    server.await.unwrap();
}

fn apply_status_identity_mismatch(status: &mut RuntimeView, mismatch: StatusIdentityMismatch) {
    match mismatch {
        StatusIdentityMismatch::RuntimeEpoch => status.runtime_epoch = Uuid::new_v4(),
        StatusIdentityMismatch::Worker => status.worker_id = Uuid::new_v4(),
        StatusIdentityMismatch::WorkerPid => {
            status.worker_pid = status.worker_pid.saturating_add(STATUS_IDENTITY_PID_OFFSET);
        }
        StatusIdentityMismatch::WorkerStart => {
            status.worker_start_time = status
                .worker_start_time
                .saturating_add(STATUS_IDENTITY_START_OFFSET);
        }
        StatusIdentityMismatch::ChildPid => {
            status.child_pid = status.child_pid.saturating_add(STATUS_IDENTITY_PID_OFFSET);
        }
        StatusIdentityMismatch::ChildStart => {
            status.child_start_time = status
                .child_start_time
                .saturating_add(STATUS_IDENTITY_START_OFFSET);
        }
    }
}
