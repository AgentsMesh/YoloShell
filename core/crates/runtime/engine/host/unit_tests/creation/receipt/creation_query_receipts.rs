use super::*;

#[tokio::test]
async fn completed_receipts_return_historical_results_without_runtime_recovery() {
    for completion in [
        CreationCompletion::Finalized,
        CreationCompletion::Survivor,
        CreationCompletion::Archived,
    ] {
        let mut fixture = QueryFixture::new();
        fixture.phase(CreationPhase::Ready);
        let receipt = fixture.receipt(completion);
        assert_eq!(
            receipt.request_fingerprint,
            fixture.binding.spec.request_fingerprint
        );
        fixture.publish(&receipt, true, true);
        fixture.remove_binding();
        let expected = match completion {
            CreationCompletion::Finalized => CreationQueryOutcome::Finalized {
                creation: fixture.binding.result,
            },
            CreationCompletion::Survivor => CreationQueryOutcome::Survivor {
                creation: fixture.binding.result,
            },
            CreationCompletion::Archived => CreationQueryOutcome::Archived {
                terminal_id: fixture.binding.result.terminal_id,
            },
        };
        assert_eq!(fixture.outcome().await, expected);
        let loaded = CreationLedger::open(&fixture.ledger_path).unwrap();
        assert_eq!(
            loaded
                .query_creation(
                    fixture.request.creation_token,
                    &CreationSpec::fingerprint(&fixture.request).unwrap(),
                    &fixture.host.creation_receipts
                )
                .unwrap()
                .outcome,
            expected
        );
    }
}

#[tokio::test]
async fn query_accepts_consistent_partial_indexes_without_repairing_them() {
    for token_index in [false, true] {
        let mut fixture = QueryFixture::new();
        fixture.phase(CreationPhase::Ready);
        let receipt = fixture.receipt(CreationCompletion::Finalized);
        fixture.publish(&receipt, token_index, !token_index);
        std::fs::write(
            fixture
                .host
                .creation_receipts
                .staging
                .path
                .join(QUERY_STAGE_FILE),
            QUERY_STAGE_BYTES,
        )
        .unwrap();
        assert_eq!(
            fixture.outcome().await,
            CreationQueryOutcome::Finalized {
                creation: fixture.binding.result
            }
        );
        let store = &fixture.host.creation_receipts;
        assert_eq!(
            store
                .lookup_token_digest(&receipt.creation_token_digest)
                .unwrap()
                .is_some(),
            token_index
        );
        assert_eq!(
            store
                .lookup_session(receipt.result.terminal_id)
                .unwrap()
                .is_some(),
            !token_index
        );
        assert_eq!(
            lock_recover(&fixture.host.creation_capabilities)
                .bindings
                .len(),
            QUERY_ONLY_BINDING
        );
    }
}

#[tokio::test]
async fn unsupported_receipts_reject_query_and_finalize_without_rewriting_evidence() {
    for completion in [CreationCompletion::Finalized, CreationCompletion::Survivor] {
        let mut fixture = QueryFixture::new();
        fixture.phase(CreationPhase::Ready);
        let mut receipt = fixture.receipt(completion);
        receipt.schema_version = CREATION_RECEIPT_SCHEMA_VERSION - QUERY_SCHEMA_OFFSET;
        let store = &fixture.host.creation_receipts;
        for path in [
            store.token_index.path.join(&receipt.creation_token_digest),
            store
                .session_index
                .path
                .join(receipt.result.terminal_id.to_string()),
        ] {
            std::fs::write(&path, serde_json::to_vec(&receipt).unwrap()).unwrap();
            set_receipt_path_mode(&path, CREATION_RECEIPT_FILE_MODE);
        }
        let before = fixture.snapshot();
        assert!(fixture.observe(&fixture.request).await.is_err());
        assert!(
            lock_recover(&fixture.host.creation_capabilities)
                .finalize(
                    &fixture.ledger_path,
                    fixture.finalize_request(),
                    &fixture.host.creation_receipts,
                )
                .is_err()
        );
        assert_eq!(fixture.snapshot(), before);
    }
}

#[tokio::test]
async fn receipt_fingerprint_and_index_conflicts_never_return_a_capability() {
    for conflict_indexes in [false, true] {
        let mut fixture = QueryFixture::new();
        fixture.phase(CreationPhase::Ready);
        let mut changed = fixture.receipt(CreationCompletion::Finalized);
        changed.request_fingerprint = yolo_crypto::sha256_hex(OTHER_FINGERPRINT);
        fixture.publish(&changed, true, !conflict_indexes);
        if conflict_indexes {
            fixture.publish(&fixture.receipt(CreationCompletion::Finalized), false, true);
        }
        let error = fixture.observe(&fixture.request).await.unwrap_err();
        assert_eq!(
            error.code,
            if conflict_indexes {
                crate::protocol::error_code::INTERNAL
            } else {
                crate::protocol::error_code::CONFLICT
            }
        );
    }
}
