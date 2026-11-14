use super::{image_faults::faults, recovery_constants::deadline, recovery_fixture::Fixture};
use crate::recover;
use yolo_remote_provisioning_protocol::EnrollmentOutcome;

#[tokio::test]
async fn every_live_process_image_must_match_before_readonly_recovery_confirms_enrollment() {
    for fault in faults() {
        let mut fixture = Fixture::new(false).await;
        let permit = fixture.prepare();
        fixture.apply(permit.prepared());
        drop(permit);
        let before = std::fs::read(fixture.file()).unwrap();
        let expected = fixture.shared.lock().unwrap().build.clone();
        fixture.shared.lock().unwrap().build = fault.changed(&expected);
        let unknown = recover(&fixture.directories, &fixture.request, deadline())
            .await
            .unwrap();
        assert!(
            matches!(unknown.fields().outcome, EnrollmentOutcome::Unknown {}),
            "{fault:?}"
        );
        assert_eq!(std::fs::read(fixture.file()).unwrap(), before, "{fault:?}");
        assert_eq!(fixture.configured(), 0, "{fault:?}");

        fixture.shared.lock().unwrap().build = expected.clone();
        let ready = recover(&fixture.directories, &fixture.request, deadline())
            .await
            .unwrap();
        let EnrollmentOutcome::Ready { receipt } = &ready.fields().outcome else {
            panic!("matching independent process proofs did not confirm enrollment: {fault:?}");
        };
        receipt.validate_for(&fixture.request).unwrap();
        assert_eq!(receipt.fields().build, expected);
        assert_eq!(std::fs::read(fixture.file()).unwrap(), before, "{fault:?}");
        assert_eq!(fixture.configured(), 0, "{fault:?}");
        fixture.peer.finish().await;
    }
}
