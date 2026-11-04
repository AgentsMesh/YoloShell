use super::{
    fake::{Fake, Fault},
    fixtures::stage,
};
use uuid::Uuid;
use yolo_distribution_bundle::ArtifactRole;
use yolo_distribution_publication_protocol::{PublicationRequest, Selection};
use yolo_remote::ChannelPurpose;
use yolo_remote_bundle::RemoteBundleInstaller;

#[tokio::test]
async fn invalid_remote_probes_or_channels_never_stage_or_commit() {
    for fault in [
        Fault::WrongPlatform,
        Fault::WrongArchitecture,
        Fault::Timeout,
        Fault::Truncated,
        Fault::MissingExit,
        Fault::WrongChannel(ChannelPurpose::Command),
        Fault::WrongChannel(ChannelPurpose::FileRead),
        Fault::MalformedSelection,
    ] {
        let connection = Fake::new(fault);
        assert!(
            RemoteBundleInstaller
                .install(&connection, &stage(), Uuid::new_v4())
                .await
                .is_err()
        );
        assert_eq!(connection.stages(), 0);
        assert_eq!(connection.commits(), 0);
    }
}

#[tokio::test]
async fn wrong_stage_or_any_wrong_executable_version_never_dispatches_commit() {
    for fault in [
        Fault::StageFailure,
        Fault::WrongStage,
        Fault::WrongChannel(ChannelPurpose::RuntimeDeployment),
        Fault::WrongVersion(ArtifactRole::Runtime),
        Fault::UnverifiedStaged(ArtifactRole::Runtime),
        Fault::WrongVerificationChannel,
    ] {
        let connection = Fake::new(fault);
        assert!(
            RemoteBundleInstaller
                .install(&connection, &stage(), Uuid::new_v4())
                .await
                .is_err()
        );
        assert_eq!(connection.commits(), 0);
    }
}

#[tokio::test]
async fn stale_selector_or_request_correlation_fails_before_upload() {
    let connection = Fake::new(Fault::None);
    let stage = stage();
    let request = RemoteBundleInstaller
        .prepare(&connection, &stage, Uuid::new_v4())
        .await
        .unwrap();
    connection.set_selected(Some(
        Selection::next(None, Uuid::new_v4(), stage.descriptor().build_id()).unwrap(),
    ));
    assert!(
        RemoteBundleInstaller
            .publish(&connection, &stage, &request)
            .await
            .is_err()
    );
    assert_eq!(connection.stages(), 0);
    let mismatch = PublicationRequest::new(
        stage.install_root().to_owned(),
        Uuid::new_v4(),
        Uuid::new_v4(),
        stage.descriptor().clone(),
        None,
    )
    .unwrap();
    assert!(
        RemoteBundleInstaller
            .publish(&connection, &stage, &mismatch)
            .await
            .is_err()
    );
    assert!(
        RemoteBundleInstaller
            .recover(&connection, &stage, &mismatch)
            .await
            .is_err()
    );
    assert_eq!(connection.commits(), 0);
}

#[tokio::test]
async fn persisted_request_cannot_publish_or_inspect_a_different_installation_root() {
    let connection = Fake::new(Fault::None);
    let original = stage();
    let request = RemoteBundleInstaller
        .prepare(&connection, &original, Uuid::new_v4())
        .await
        .unwrap();
    let retargeted = yolo_remote::BundleStageRequest::new(
        super::constants::OTHER_ROOT.into(),
        original.stage_id(),
        original.descriptor().clone(),
        yolo_remote::BundlePayloads {
            runtime: original.payload(ArtifactRole::Runtime).into(),
        },
    )
    .unwrap();
    assert!(
        RemoteBundleInstaller
            .publish(&connection, &retargeted, &request)
            .await
            .is_err()
    );
    assert!(
        RemoteBundleInstaller
            .recover(&connection, &retargeted, &request)
            .await
            .is_err()
    );
    assert!(connection.commands().is_empty());
    assert_eq!(connection.stages(), 0);
    assert_eq!(connection.verifications(), 0);
}
