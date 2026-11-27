use super::{constants::*, fixture::*};
use yolo_remote_provisioning_protocol::{ProvisionOutcome, ProvisionPhase};

#[tokio::test]
async fn complete_publish_and_enroll_once_with_live_readonly_recovery_on_reentry() {
    let fixture = Fixture::new();
    let service = fixture.service();
    let result = service
        .provision(&fixture.remote, &authorized, &fixture.request, deadline())
        .await
        .unwrap();
    assert!(
        matches!(result.fields().outcome, ProvisionOutcome::Ready { .. }),
        "unexpected provisioning outcome: {:?}",
        result.fields().outcome
    );
    assert_eq!(fixture.remote.bundle.commits(), ONE);
    assert_eq!(fixture.remote.state.lock().unwrap().calls, ONE);
    let bytes = std::fs::read(fixture.file()).unwrap();
    let reconnected = fixture.remote.reconnect();
    assert!(matches!(
        service
            .provision(&reconnected, &authorized, &fixture.request, deadline())
            .await
            .unwrap()
            .fields()
            .outcome,
        ProvisionOutcome::Ready { .. }
    ));
    assert_eq!(fixture.remote.bundle.commits(), ONE);
    assert_eq!(fixture.remote.state.lock().unwrap().calls, ONE);
    assert_eq!(std::fs::read(fixture.file()).unwrap(), bytes);
    fixture.remote.state.lock().unwrap().superseded = true;
    assert!(matches!(
        service
            .recover(&reconnected, &authorized, &fixture.request, deadline())
            .await
            .unwrap()
            .fields()
            .outcome,
        ProvisionOutcome::Superseded { .. }
    ));
}

#[tokio::test]
async fn enrollment_lost_reply_preserves_unknown_and_never_replays_on_new_generation() {
    let fixture = Fixture::new();
    let service = fixture.service();
    fixture.remote.state.lock().unwrap().lose_reply = true;
    let result = service
        .provision(&fixture.remote, &authorized, &fixture.request, deadline())
        .await
        .unwrap();
    assert!(matches!(
        result.fields().outcome,
        ProvisionOutcome::Unknown {
            phase: ProvisionPhase::Enrolling
        }
    ));
    let before = std::fs::read(fixture.file()).unwrap();
    let next = fixture.remote.reconnect();
    assert!(matches!(
        service
            .recover(&next, &authorized, &fixture.request, deadline())
            .await
            .unwrap()
            .fields()
            .outcome,
        ProvisionOutcome::Ready { .. }
    ));
    assert!(
        service
            .continue_known_stage(&next, &authorized, &fixture.request, deadline())
            .await
            .is_err()
    );
    assert_eq!(fixture.remote.state.lock().unwrap().calls, ONE);
    assert_eq!(std::fs::read(fixture.file()).unwrap(), before);
}

#[tokio::test]
async fn published_known_stage_requires_explicit_continue_and_gets_only_one_enrollment_permit() {
    let fixture = Fixture::new();
    let service = fixture.service();
    let guard = || {
        if let Ok(bytes) = std::fs::read(fixture.file()) {
            let record: serde_json::Value = serde_json::from_slice(&bytes)?;
            anyhow::ensure!(record[PHASE_FIELD] != PUBLISHED_PHASE, REVOKED);
        }
        Ok(())
    };
    assert!(matches!(
        service
            .provision(&fixture.remote, &guard, &fixture.request, deadline())
            .await
            .unwrap()
            .fields()
            .outcome,
        ProvisionOutcome::Unknown {
            phase: ProvisionPhase::Published
        }
    ));
    assert_eq!(fixture.remote.state.lock().unwrap().calls, 0);
    assert!(matches!(
        service
            .provision(&fixture.remote, &authorized, &fixture.request, deadline())
            .await
            .unwrap()
            .fields()
            .outcome,
        ProvisionOutcome::Published { .. }
    ));
    assert_eq!(fixture.remote.state.lock().unwrap().calls, 0);
    assert!(matches!(
        service
            .continue_known_stage(&fixture.remote, &authorized, &fixture.request, deadline())
            .await
            .unwrap()
            .fields()
            .outcome,
        ProvisionOutcome::Ready { .. }
    ));
    assert!(
        service
            .continue_known_stage(&fixture.remote, &authorized, &fixture.request, deadline())
            .await
            .is_err()
    );
    assert_eq!(fixture.remote.bundle.commits(), ONE);
    assert_eq!(fixture.remote.state.lock().unwrap().calls, ONE);
}

#[tokio::test]
async fn ambiguous_publishing_never_uploads_or_bootstraps_again() {
    let fixture = Fixture::new();
    let service = fixture.service();
    let guard = || {
        if let Ok(bytes) = std::fs::read(fixture.file()) {
            let record: serde_json::Value = serde_json::from_slice(&bytes)?;
            anyhow::ensure!(record[PHASE_FIELD] != PUBLISHING_PHASE, REVOKED);
        }
        Ok(())
    };
    let result = service
        .provision(&fixture.remote, &guard, &fixture.request, deadline())
        .await
        .unwrap();
    assert!(matches!(
        result.fields().outcome,
        ProvisionOutcome::Unknown {
            phase: ProvisionPhase::Publishing
        }
    ));
    assert_eq!(fixture.remote.bundle.stages(), 0);
    let before = std::fs::read(fixture.file()).unwrap();
    assert!(matches!(
        service
            .provision(&fixture.remote, &authorized, &fixture.request, deadline())
            .await
            .unwrap()
            .fields()
            .outcome,
        ProvisionOutcome::Unknown {
            phase: ProvisionPhase::Publishing
        }
    ));
    assert!(
        service
            .continue_known_stage(&fixture.remote, &authorized, &fixture.request, deadline())
            .await
            .is_err()
    );
    assert_eq!(fixture.remote.bundle.stages(), 0);
    assert_eq!(fixture.remote.state.lock().unwrap().calls, 0);
    assert_eq!(std::fs::read(fixture.file()).unwrap(), before);
}
