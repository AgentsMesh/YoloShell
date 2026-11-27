use yolo_hub_remote_startup_test_support::*;
use yolo_remote_provisioning_protocol::*;

#[test]
fn recovery_outcomes_are_strict_and_bound_to_the_original_request_and_receipt() {
    let request = request();
    let receipt = RemoteBootstrapReceipt::new(receipt(&request)).unwrap();
    for outcome in [
        EnrollmentOutcome::Absent {},
        EnrollmentOutcome::Unknown {},
        EnrollmentOutcome::Superseded {},
        EnrollmentOutcome::Ready { receipt },
    ] {
        let recovery = EnrollmentRecovery::new(&request, outcome).unwrap();
        recovery.validate_for(&request).unwrap();
        assert_eq!(
            serde_json::from_slice::<EnrollmentRecovery>(&serde_json::to_vec(&recovery).unwrap())
                .unwrap(),
            recovery
        );
        let mut fields = request.fields().clone();
        fields.request_id = uuid::Uuid::new_v4();
        assert!(
            recovery
                .validate_for(&RemoteBootstrapRequest::new(fields).unwrap())
                .is_err()
        );
        let mut fields = recovery.fields().clone();
        fields.request_id = uuid::Uuid::nil();
        assert!(EnrollmentRecovery::try_from(fields).is_err());
        let mut fields = recovery.fields().clone();
        fields.schema_version += 1;
        assert!(EnrollmentRecovery::try_from(fields).is_err());
    }
}
