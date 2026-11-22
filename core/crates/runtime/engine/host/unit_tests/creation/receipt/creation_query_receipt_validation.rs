use super::*;

#[test]
fn only_current_receipts_with_valid_fingerprints_are_readable() {
    let fixture = QueryFixture::new();
    let current = fixture.receipt(CreationCompletion::Archived);
    for fingerprint in [String::new(), OTHER_NAME.to_owned()] {
        let mut changed = current.clone();
        changed.request_fingerprint = fingerprint;
        assert!(changed.validate().is_err());
    }
    let mut unsupported = current.clone();
    for schema in 0..CREATION_RECEIPT_SCHEMA_VERSION {
        unsupported.schema_version = schema;
        assert!(unsupported.validate().is_err());
    }
    let mut missing = serde_json::to_value(&current).unwrap();
    missing.as_object_mut().unwrap().remove(FINGERPRINT_FIELD);
    assert!(serde_json::from_value::<CreationReceipt>(missing).is_err());
}

#[test]
fn receipt_recovery_cannot_remove_a_binding_for_another_request_fingerprint() {
    let mut fixture = QueryFixture::new();
    fixture.phase(CreationPhase::Ready);
    let mut receipt = fixture.receipt(CreationCompletion::Finalized);
    receipt.request_fingerprint = yolo_crypto::sha256_hex(OTHER_FINGERPRINT);
    fixture.publish(&receipt, true, true);
    let before = std::fs::read(&fixture.ledger_path).unwrap();
    let mut ledger = lock_recover(&fixture.host.creation_capabilities);
    let original = ledger.clone();
    assert!(
        ledger
            .recover_completed(&fixture.ledger_path, &fixture.host.creation_receipts)
            .is_err()
    );
    assert_eq!(*ledger, original);
    assert_eq!(std::fs::read(&fixture.ledger_path).unwrap(), before);
}

#[tokio::test]
async fn finalize_rejects_conflicting_v2_fingerprints_before_publication_or_binding_retirement() {
    for completion in [CreationCompletion::Finalized, CreationCompletion::Survivor] {
        let mut fixture = QueryFixture::new();
        fixture.phase(CreationPhase::Ready);
        let mut receipt = fixture.receipt(completion);
        receipt.request_fingerprint = yolo_crypto::sha256_hex(OTHER_FINGERPRINT);
        fixture.publish(&receipt, true, true);
        let before = fixture.snapshot();
        let request = fixture.finalize_request();
        let expected = lock_recover(&fixture.host.creation_capabilities).clone();
        // The public Host path must reject the existing receipt in pending_finalize.
        let error = fixture.host.finalize_creation(request).await.unwrap_err();
        assert_eq!(error.code, crate::protocol::error_code::INTERNAL);
        assert_eq!(fixture.snapshot(), before);
        let mut ledger = lock_recover(&fixture.host.creation_capabilities);
        assert_eq!(*ledger, expected);
        // The commit path must repeat the check even if preflight ran earlier.
        let error = ledger
            .finalize(
                &fixture.ledger_path,
                fixture.finalize_request(),
                &fixture.host.creation_receipts,
            )
            .unwrap_err();
        assert_eq!(error.code, crate::protocol::error_code::INTERNAL);
        assert_eq!(*ledger, expected);
        drop(ledger);
        assert_eq!(fixture.snapshot(), before);
    }
}
