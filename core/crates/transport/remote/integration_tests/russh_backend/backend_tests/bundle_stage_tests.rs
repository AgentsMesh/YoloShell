use super::{bundle_fixture::*, memory_fs::FailurePoint};
use crate::remote::*;
use yolo_distribution_bundle::{ArtifactRole, MANIFEST_FILE_NAME};
use yolo_distribution_publication_protocol::{
    CURRENT_FILE_NAME, DIRECTORY_MODE, EXECUTABLE_MODE, MANIFEST_MODE,
};

#[tokio::test]
async fn bundle_stages_all_roles_privately_without_touching_selector_and_never_reuses_stage() {
    let fixture = fixture().await;
    let request = request();
    let staged = fixture.connection.stage_bundle(&request).await.unwrap();
    assert_eq!(
        staged.identity.connection_id,
        fixture.connection.info().connection_id
    );
    assert_eq!(staged.identity.purpose, ChannelPurpose::RuntimeDeployment);
    assert_eq!(staged.value.descriptor(), request.descriptor());
    for role in ArtifactRole::ALL {
        assert_eq!(staged.value.executable(role), request.executable(role));
        assert_eq!(
            fixture
                .filesystem
                .permissions(request.executable(role).to_str().unwrap()),
            Some(EXECUTABLE_MODE)
        );
    }
    assert_eq!(
        fixture.filesystem.permissions(
            request
                .stage_directory()
                .join(MANIFEST_FILE_NAME)
                .to_str()
                .unwrap()
        ),
        Some(MANIFEST_MODE)
    );
    assert!(
        !fixture.filesystem.contains_file(
            request
                .install_root()
                .join(CURRENT_FILE_NAME)
                .to_str()
                .unwrap()
        )
    );
    assert_eq!(fixture.filesystem.activations(), usize::default());
    assert!(fixture.connection.stage_bundle(&request).await.is_err());
    assert!(
        fixture
            .filesystem
            .contains_file(request.executable(ArtifactRole::Runtime).to_str().unwrap())
    );
}

#[tokio::test]
async fn failed_bundle_write_retains_unproven_stage_and_preserves_existing_selection() {
    let fixture = fixture().await;
    fixture.filesystem.insert_directory(ROOT, DIRECTORY_MODE);
    let selector = std::path::Path::new(ROOT).join(CURRENT_FILE_NAME);
    fixture
        .filesystem
        .insert_file(selector.to_str().unwrap(), SELECTION_BYTES, MANIFEST_MODE);
    fixture.filesystem.fail_once(FailurePoint::UploadWrite);
    let request = request();
    let error = fixture.connection.stage_bundle(&request).await.unwrap_err();
    assert!(
        matches!(error, RemoteError::BundleStageRetained { ref stage, .. } if stage.stage_directory() == request.stage_directory())
    );
    assert!(
        fixture
            .filesystem
            .metadata(request.stage_directory().to_str().unwrap(), false)
            .is_ok()
    );
    assert_eq!(fixture.connection.cleanup.retained_bundle_stages().len(), 1);
    assert_eq!(
        fixture
            .connection
            .read_bundle_selection(std::path::Path::new(ROOT))
            .await
            .unwrap()
            .value,
        Some(SELECTION_BYTES.to_vec())
    );
}

#[tokio::test]
async fn cancelling_upload_settles_its_channel_and_reports_retained_stage_without_deletion() {
    let fixture = fixture().await;
    fixture
        .filesystem
        .fail_once(FailurePoint::SftpOpenResponsePending);
    let request = request();
    let path = request.stage_directory();
    let connection = fixture.connection.clone();
    let running = tokio::spawn(async move { connection.stage_bundle(&request).await.map(|_| ()) });
    tokio::time::timeout(super::constants::TEST_INACTIVITY_TIMEOUT, async {
        while fixture.filesystem.live_handles() == 0 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    running.abort();
    assert!(running.await.unwrap_err().is_cancelled());
    fixture
        .connection
        .cleanup
        .settle(cleanup_settlement_timeout(
            super::constants::NORMAL_OPERATION_TIMEOUT,
        ))
        .await
        .unwrap();
    assert!(
        fixture
            .filesystem
            .metadata(path.to_str().unwrap(), false)
            .is_ok()
    );
    let retained = fixture.connection.cleanup.retained_bundle_stages();
    assert_eq!(retained.len(), 1);
    assert_eq!(retained[0].stage_directory(), path);
    assert_eq!(retained[0].reason(), BundleRetentionReason::Cancelled);
    assert!(!fixture.connection.session.is_closed());
}
