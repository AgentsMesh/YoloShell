use super::{
    bundle_fixture::{self, *},
    constants::*,
    hub_peer_fixture::{self, PeerFixture},
    memory_fs::FailurePoint,
};
use crate::remote::*;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    time::timeout_at,
};

#[tokio::test]
async fn cancelled_readback_closes_only_its_sftp_channel_and_keeps_sibling_hub_peer_usable() {
    let fixture = PeerFixture::new().await;
    fixture.ssh.filesystem.insert_directory(PARENT, PARENT_MODE);
    let mut sibling = fixture
        .ssh
        .connection
        .open_hub_peer(&fixture.request)
        .await
        .unwrap();
    let (mut socket, _) = timeout_at(hub_peer_fixture::deadline(), fixture.listener.accept())
        .await
        .unwrap()
        .unwrap();
    fixture
        .ssh
        .filesystem
        .fail_once(FailurePoint::BundleReadPending);
    let stage = request();
    let path = stage.stage_directory();
    let connection = fixture.ssh.connection.clone();
    let running = tokio::spawn(async move { connection.stage_bundle(&stage).await.map(|_| ()) });
    timeout_at(hub_peer_fixture::deadline(), async {
        while fixture
            .ssh
            .filesystem
            .failure_is_armed(FailurePoint::BundleReadPending)
        {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    running.abort();
    assert!(running.await.unwrap_err().is_cancelled());
    wait_retained(&fixture.ssh.connection).await;
    sibling
        .stream
        .write_all(hub_peer_fixture::PAYLOAD)
        .await
        .unwrap();
    let mut bytes = vec![u8::default(); hub_peer_fixture::PAYLOAD.len()];
    timeout_at(hub_peer_fixture::deadline(), socket.read_exact(&mut bytes))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(bytes, hub_peer_fixture::PAYLOAD);
    hub_peer_fixture::assert_connection_healthy(&fixture.ssh).await;
    drop(sibling);
    fixture
        .ssh
        .connection
        .cleanup
        .settle(cleanup_settlement_timeout(NORMAL_OPERATION_TIMEOUT))
        .await
        .unwrap();
    assert!(
        fixture
            .ssh
            .filesystem
            .metadata(path.to_str().unwrap(), false)
            .is_ok()
    );
    assert_eq!(
        fixture.ssh.connection.cleanup.retained_bundle_stages()[0].reason(),
        BundleRetentionReason::Cancelled
    );
}

#[derive(Clone, Copy)]
enum Replacement {
    SameBytes,
    DifferentBytes,
    Symlink,
}

#[tokio::test]
async fn cancellation_never_deletes_same_name_replacements_even_when_their_bytes_match() {
    use russh_sftp::protocol::{FileAttributes, OpenFlags};
    use yolo_distribution_publication_protocol::EXECUTABLE_MODE;
    for replacement in [
        Replacement::SameBytes,
        Replacement::DifferentBytes,
        Replacement::Symlink,
    ] {
        let fixture = bundle_fixture::fixture().await;
        fixture
            .filesystem
            .fail_once(FailurePoint::SftpOpenResponsePending);
        let stage = request();
        let path = stage.executable(ArtifactRole::Runtime);
        let connection = fixture.connection.clone();
        let copied = stage.clone();
        let running =
            tokio::spawn(async move { connection.stage_bundle(&copied).await.map(|_| ()) });
        timeout_at(hub_peer_fixture::deadline(), async {
            while fixture.filesystem.live_handles() == 0 {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        let name = path.to_str().unwrap();
        let retained_name = match replacement {
            Replacement::SameBytes => {
                fixture.filesystem.insert_file(
                    name,
                    stage.payload(ArtifactRole::Runtime),
                    EXECUTABLE_MODE,
                );
                name
            }
            Replacement::DifferentBytes => {
                fixture
                    .filesystem
                    .insert_file(name, REPLACEMENT_FILE_CONTENT, EXECUTABLE_MODE);
                name
            }
            Replacement::Symlink => {
                fixture.filesystem.remove(name);
                fixture.filesystem.insert_link(name, MEMORY_OUTSIDE);
                MEMORY_OUTSIDE
            }
        };
        let retained_file = fixture
            .filesystem
            .open(retained_name, OpenFlags::READ, FileAttributes::empty())
            .unwrap();
        running.abort();
        assert!(running.await.unwrap_err().is_cancelled());
        fixture
            .connection
            .cleanup
            .settle(cleanup_settlement_timeout(NORMAL_OPERATION_TIMEOUT))
            .await
            .unwrap();
        assert!(
            fixture
                .filesystem
                .same_file_at(retained_name, &retained_file)
        );
        if matches!(replacement, Replacement::Symlink) {
            assert_eq!(
                fixture.filesystem.target(name).as_deref(),
                Some(MEMORY_OUTSIDE)
            );
        }
        let report = fixture.connection.cleanup.retained_bundle_stages();
        assert_eq!(report.len(), ONE_RECORD);
        assert_eq!(report[0].stage_id(), stage.stage_id());
        assert_eq!(report[0].build_id(), stage.descriptor().build_id());
        assert_eq!(
            report[0].connection_id(),
            fixture.connection.info().connection_id
        );
        assert_eq!(report[0].install_root(), stage.install_root());
    }
}

async fn wait_retained(connection: &RusshConnection) {
    timeout_at(hub_peer_fixture::deadline(), async {
        while connection.cleanup.retained_bundle_stages().is_empty() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
}

const ONE_RECORD: usize = 1;
