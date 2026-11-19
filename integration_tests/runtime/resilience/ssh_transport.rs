use super::*;
use std::sync::Arc;
use tokio::time::timeout;
use yolo_remote::{CommandPolicy, FileReadPolicy, RemoteEndpoint, RemoteRuntimeSpec, RuntimeRole};
use yolo_remote_broker::BrokerConnectionPool;
use yolo_runtime::model::{
    CreateTerminal, CreateTerminalResult, FinalizeCreationRequest, HostStatus, TerminalView,
};
use yolo_runtime::protocol::method;
use yolo_runtime::worker::{AttachRequest, AttachResult};

#[path = "ssh_transport/constants.rs"]
mod constants;
#[path = "ssh_transport/factory.rs"]
mod factory;
#[path = "ssh_transport/operations.rs"]
mod operations;
#[path = "ssh_transport/proxy.rs"]
mod proxy;
#[path = "ssh_transport/rpc.rs"]
mod rpc;
#[path = "ssh_transport/runtime_guard.rs"]
mod runtime_guard;
#[path = "ssh_transport/setup.rs"]
mod setup;
#[path = "ssh_transport/sshd.rs"]
mod sshd;

use constants::*;
pub(crate) use factory::LiveSshFactory;
use operations::{assert_multiplexed, write_terminal_marker};
use proxy::TransportProxy;
use rpc::*;
use setup::open_initial;
pub(crate) use sshd::SshdFixture;

#[cfg(target_os = "macos")]
#[path = "ssh_transport/runtime_guard_wiring_tests.rs"]
mod runtime_guard_wiring_tests;

#[cfg(target_os = "macos")]
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn open_ssh_reset_preserves_remote_pty_and_broker_reconnects() {
    let sshd = SshdFixture::start().await;
    let proxy = TransportProxy::start(sshd.address()).await;
    let endpoint = RemoteEndpoint::new(sshd.user(), LOOPBACK_HOST, proxy.port()).unwrap();
    let factory = LiveSshFactory::new(endpoint, sshd.known_hosts(proxy.port()), sshd.identity());
    let runtime =
        RemoteRuntimeSpec::new(runtime_binary().canonicalize().unwrap(), sshd.remote_home())
            .unwrap();
    let (connection, opened, mut guard) = open_initial(&sshd, &factory, &runtime).await;
    factory.seed(connection);
    let pool = Arc::new(BrokerConnectionPool::new(
        TEST_HOST_ID,
        TEST_CONFIG_REVISION,
        runtime,
        factory,
    ));
    let command_policy = CommandPolicy::new([PRINTF_PROGRAM.into()]).unwrap();
    let file_policy =
        FileReadPolicy::standard(vec![sshd.readable_file().parent().unwrap().to_path_buf()])
            .unwrap();
    let mut control = opened.stream;
    let before_host: HostStatus = request_at(
        &mut control,
        method::HOST_STATUS,
        &(),
        INITIAL_HOST_STATUS_STAGE,
    )
    .await;
    guard.verify_status(&before_host).unwrap();
    let creation_token = Uuid::new_v4();
    let created: CreateTerminalResult = request(
        &mut control,
        method::CREATE_SESSION,
        &CreateTerminal {
            creation_token,
            name: Some(TEST_SESSION_NAME.into()),
            cwd: sshd.remote_home().to_path_buf(),
            cols: Some(90),
            rows: Some(25),
        },
    )
    .await;
    let _: () = request(
        &mut control,
        method::FINALIZE_CREATION,
        &FinalizeCreationRequest {
            creation_token,
            terminal_id: created.terminal_id,
            creation_capability: created.creation_capability,
        },
    )
    .await;
    let before = wait_for_running(&mut control, created.terminal_id).await;
    guard.track_running_runtime(&before).unwrap();
    let mut attachment = attach(&pool, created.terminal_id, 0).await;
    assert_multiplexed(&pool, &command_policy, &file_policy, sshd.readable_file()).await;
    let first_status = pool.status().await;
    assert_eq!(
        first_status.connection_generation,
        FIRST_CONNECTION_GENERATION
    );
    let first_connection = first_status.connection_id.unwrap();

    proxy.cut_active_transport().await;
    tokio::join!(
        assert_transport_closed(&mut control),
        assert_transport_closed(&mut attachment)
    );
    let mut control = timeout(
        TRANSPORT_EVENT_TIMEOUT,
        pool.open_runtime(RuntimeRole::Control),
    )
    .await
    .expect("broker reconnect timed out")
    .unwrap();
    let reconnected = pool.status().await;
    assert_eq!(reconnected.connection_generation, RECONNECTED_GENERATION);
    assert_ne!(reconnected.connection_id.unwrap(), first_connection);
    let after_host: HostStatus = request_at(
        &mut control,
        method::HOST_STATUS,
        &(),
        RECONNECTED_HOST_STATUS_STAGE,
    )
    .await;
    guard.verify_status(&after_host).unwrap();
    setup::bind_runtime_inventory(&mut control, &mut guard)
        .await
        .unwrap();
    assert_eq!(after_host.host_pid, before_host.host_pid);
    assert_eq!(after_host.boot_id, before_host.boot_id);
    let after = wait_for_running(&mut control, created.terminal_id).await;
    assert_runtime_identity(&before, &after);

    let after_sequence = after.runtime.as_ref().unwrap().last_output_sequence;
    let mut attachment = attach(&pool, created.terminal_id, after_sequence).await;
    assert_multiplexed(&pool, &command_policy, &file_policy, sshd.readable_file()).await;
    write_terminal_marker(&mut control, runtime_ref_from_view(&after)).await;
    wait_for_terminal_marker(&mut attachment, PTY_MARKER).await;
    let final_view = wait_for_running(&mut control, created.terminal_id).await;
    assert_runtime_identity(&before, &final_view);
    drop((control, attachment));
    guard.shutdown().await.unwrap();
}

async fn attach(
    pool: &BrokerConnectionPool,
    terminal_id: Uuid,
    after_seq: u64,
) -> yolo_runtime_rpc::RuntimeConnection {
    let mut stream = pool.open_runtime(RuntimeRole::Attachment).await.unwrap();
    let _: AttachResult = request(
        &mut stream,
        method::ATTACH_SESSION,
        &AttachRequest {
            terminal_id,
            after_seq,
        },
    )
    .await;
    stream
}

fn assert_runtime_identity(before: &TerminalView, after: &TerminalView) {
    let before = before.runtime.as_ref().unwrap();
    let after = after.runtime.as_ref().unwrap();
    assert_eq!(after.runtime_epoch, before.runtime_epoch);
    assert_eq!(after.worker_id, before.worker_id);
    assert_eq!(after.worker_pid, before.worker_pid);
    assert_eq!(after.child_pid, before.child_pid);
    assert_eq!(after.status, TerminalRuntimeStatus::Running);
}
