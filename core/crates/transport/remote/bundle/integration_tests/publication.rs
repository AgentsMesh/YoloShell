use super::{
    constants::*,
    fake::{Fake, Fault},
    fixtures::stage,
};
use uuid::Uuid;
use yolo_distribution_bundle::ArtifactRole;
use yolo_distribution_publication_protocol::*;
use yolo_remote_bundle::{RemoteBundleInstaller, UncertainPublication};

#[tokio::test]
async fn complete_bundle_executes_one_bound_commit_and_replay_is_read_only() {
    let connection = Fake::new(Fault::None);
    let stage = stage();
    let request = RemoteBundleInstaller
        .prepare(&connection, &stage, Uuid::new_v4())
        .await
        .unwrap();
    let receipt = RemoteBundleInstaller
        .publish(&connection, &stage, &request)
        .await
        .unwrap();
    receipt.validate(&request, stage.install_root()).unwrap();
    assert_eq!(connection.stages(), 1);
    assert_eq!(connection.commits(), 1);
    let replay = RemoteBundleInstaller
        .publish(&connection, &stage, &request)
        .await
        .unwrap();
    assert_eq!(replay, receipt);
    assert_eq!(connection.stages(), 1);
    assert_eq!(connection.commits(), 1);
    assert_eq!(connection.inspections(), 1);
    let commands = connection.commands();
    let commit = commands
        .iter()
        .find(|command| {
            command
                .args()
                .strip_prefix(
                    &yolo_launch_contract::ProcessRole::Hub
                        .arguments()
                        .map(str::to_owned),
                )
                .and_then(|arguments| arguments.first())
                .is_some_and(|arg| arg == INSTALL_BUNDLE_ARGUMENT)
        })
        .unwrap();
    assert_eq!(
        commit.program(),
        stage.executable(ArtifactRole::Runtime).to_str().unwrap()
    );
    assert!(commit.args().contains(&ROOT.to_owned()));
    let pair = commit
        .args()
        .windows(2)
        .find(|pair| pair[0] == REQUEST_CAPNP_ARGUMENT)
        .unwrap();
    assert_eq!(decode_request_argument(&pair[1]).unwrap(), request);
}

#[tokio::test]
async fn lost_or_malformed_commit_receipt_is_recovered_only_by_verified_read_only_inspection() {
    for fault in [Fault::LostAck, Fault::InvalidReceipt] {
        let connection = Fake::new(fault);
        let stage = stage();
        let request = RemoteBundleInstaller
            .prepare(&connection, &stage, Uuid::new_v4())
            .await
            .unwrap();
        let receipt = RemoteBundleInstaller
            .publish(&connection, &stage, &request)
            .await
            .unwrap();
        receipt.validate(&request, stage.install_root()).unwrap();
        assert_eq!(connection.commits(), 1);
        assert_eq!(connection.inspections(), 1);
        let command = connection
            .commands()
            .into_iter()
            .find(|command| {
                command
                    .args()
                    .strip_prefix(
                        &yolo_launch_contract::ProcessRole::Hub
                            .arguments()
                            .map(str::to_owned),
                    )
                    .and_then(|arguments| arguments.first())
                    .is_some_and(|arg| arg == INSPECT_INSTALL_ARGUMENT)
            })
            .unwrap();
        assert_eq!(
            command.program(),
            stage
                .descriptor()
                .executable_path(stage.install_root(), ArtifactRole::Runtime)
                .to_str()
                .unwrap()
        );
    }
}

#[tokio::test]
async fn a_newer_selection_uncommitted_stage_or_unverified_inspector_never_replays_commit() {
    for fault in [
        Fault::Superseded,
        Fault::FailedCommit,
        Fault::CorruptRecovery,
    ] {
        let connection = Fake::new(fault);
        let stage = stage();
        let request = RemoteBundleInstaller
            .prepare(&connection, &stage, Uuid::new_v4())
            .await
            .unwrap();
        let error = RemoteBundleInstaller
            .publish(&connection, &stage, &request)
            .await
            .unwrap_err();
        let uncertain = error.downcast_ref::<UncertainPublication>().unwrap();
        assert_eq!(uncertain.request(), &request);
        assert_eq!(connection.commits(), 1);
        assert!(
            RemoteBundleInstaller
                .recover(&connection, &stage, &request)
                .await
                .is_err()
        );
        assert_eq!(connection.commits(), 1);
        if matches!(fault, Fault::CorruptRecovery) {
            assert_eq!(connection.inspections(), 0);
        }
    }
}
