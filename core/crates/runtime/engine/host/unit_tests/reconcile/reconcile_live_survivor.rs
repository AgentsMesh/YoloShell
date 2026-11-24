use crate::host::tests::*;
use super::*;

const LIVE_SURVIVOR_STATE_COUNT: usize = 3;
const LIVE_SURVIVOR_SOCKET_EXTENSION: &str = "sock";

#[tokio::test]
async fn watchdog_keeps_every_live_survivor_manifest_uncertain() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let mut servers = Vec::new();
    let states = [
        RecoveryState::Created,
        RecoveryState::Reattached,
        RecoveryState::Uncertain,
    ];
    let mut session_ids = Vec::new();
    for state in states {
        let mut manifest = fixture_manifest(directory.path());
        manifest.recovery_state = state;
        let socket = directory
            .path()
            .join(manifest.terminal_id.to_string())
            .with_extension(LIVE_SURVIVOR_SOCKET_EXTENSION);
        let listener = UnixListener::bind(&socket).unwrap();
        let mut record = host_runtime_record(&host, manifest.terminal_id, socket);
        record.bootstrap_delivery = BootstrapDelivery::Survivor;
        let status = status_view(&record, Some(Vec::new()));
        persist_host_session(&host, &manifest, &record);
        session_ids.push(manifest.terminal_id);
        servers.push(tokio::spawn(fake_status_server(
            listener,
            record,
            Some(status),
        )));
    }

    let result = host.reconcile().await;
    assert_eq!(result.reattached, LIVE_SURVIVOR_STATE_COUNT);
    for server in servers {
        server.await.unwrap();
    }
    for terminal_id in session_ids {
        assert_uncertain_survivor_recovery(&host, terminal_id);
    }
}
