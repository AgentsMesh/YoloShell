use super::bundle_fixture::{fixture, request};
use crate::remote::*;

const LOST_CONNECTION: &str = "injected lost bundle SSH connection";

#[tokio::test]
async fn retention_capacity_is_reserved_before_io_and_unused_preflight_slots_are_released() {
    let fixture = fixture().await;
    for _ in 0..=MAX_RETAINED_BUNDLE_STAGES {
        drop(
            fixture
                .connection
                .cleanup
                .reserve_stage(fixture.connection.info().connection_id, &request())
                .unwrap(),
        );
    }
    for _ in 0..MAX_RETAINED_BUNDLE_STAGES {
        let slot = fixture
            .connection
            .cleanup
            .reserve_stage(fixture.connection.info().connection_id, &request())
            .unwrap();
        slot.attempted();
        slot.retain(BundleRetentionReason::Failed).unwrap();
    }
    assert!(fixture.connection.stage_bundle(&request()).await.is_err());
    assert_eq!(fixture.filesystem.uid_probes(), usize::default());
    assert_eq!(fixture.filesystem.writes(), usize::default());
    let reports = fixture.connection.cleanup.retained_bundle_stages();
    assert_eq!(reports.len(), MAX_RETAINED_BUNDLE_STAGES);
    let error = RemoteError::BundleStageRetained {
        stage: reports[0].clone(),
        cause: Box::new(RemoteError::transport(LOST_CONNECTION)),
    };
    assert!(error.is_connection_loss());
    assert!(error.requires_connection_recovery());
}
