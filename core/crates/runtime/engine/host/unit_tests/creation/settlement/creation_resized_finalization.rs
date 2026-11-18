use crate::host::tests::*;
use super::*;

#[tokio::test]
async fn resized_ready_creation_finalizes_queries_and_replays_without_replacing_its_runtime() {
    use crate::model::CreationQueryOutcome;
    let (_directory, host, request, binding, runtime) = resized_ready_creation_fixture().await;
    let session = binding.result.terminal_id;
    let runtime_path = host.paths.runtime_record(session);
    let launch_path = host.paths.worker_launch(session);
    let manifest_path = host.paths.manifest(session);
    let runtime_before = std::fs::read(&runtime_path).unwrap();
    let launch_before = std::fs::read(&launch_path).unwrap();
    let manifest_before = std::fs::read(&manifest_path).unwrap();
    let launch: WorkerLaunchSpec = read_json(&launch_path).unwrap();
    assert_eq!(
        (launch.cols, launch.rows),
        (CREATION_TEST_COLUMNS, CREATION_TEST_ROWS)
    );
    assert_eq!(
        (runtime.cols, runtime.rows),
        (RESIZED_CREATION_COLUMNS, RESIZED_CREATION_ROWS)
    );
    assert_eq!(
        host.query_creation(request.clone()).await.unwrap().outcome,
        CreationQueryOutcome::Ready {
            creation: binding.result
        }
    );
    // The fixture has no usable worker executable: replay must retain the
    // original launch, never create or start another process.
    assert_eq!(
        host.create_session(request.clone()).await.unwrap(),
        binding.result
    );
    let finalize = finalize_request_for(request.creation_token, &binding);
    host.finalize_creation(finalize).await.unwrap();
    host.finalize_creation(finalize).await.unwrap();
    assert!(
        lock_recover(&host.creation_capabilities)
            .bindings
            .is_empty()
    );
    assert_eq!(
        host.query_creation(request.clone()).await.unwrap().outcome,
        CreationQueryOutcome::Finalized {
            creation: binding.result
        }
    );
    // Only Finalize is replayable after its receipt retires the creation token.
    assert_eq!(
        host.create_session(request.clone()).await.unwrap_err().code,
        crate::protocol::error_code::CONFLICT
    );
    let mut changed_spec = request;
    changed_spec.cols = Some(runtime.cols);
    assert_eq!(
        host.query_creation(changed_spec).await.unwrap_err().code,
        crate::protocol::error_code::CONFLICT
    );
    assert_eq!(std::fs::read(runtime_path).unwrap(), runtime_before);
    assert_eq!(std::fs::read(launch_path).unwrap(), launch_before);
    assert_eq!(std::fs::read(manifest_path).unwrap(), manifest_before);
}

#[tokio::test]
async fn canonical_runtime_dimension_boundaries_remain_valid_creation_evidence() {
    let (_directory, host, _, binding, runtime) = resized_ready_creation_fixture().await;
    let planned = binding.launch.as_ref().unwrap();
    for (cols, rows) in [
        (
            crate::model::MIN_TERMINAL_COLUMNS,
            crate::model::MIN_TERMINAL_ROWS,
        ),
        (u16::MAX, u16::MAX),
    ] {
        let mut candidate = runtime.clone();
        candidate.cols = cols;
        candidate.rows = rows;
        atomic_write_json(
            &host.paths.runtime_record(binding.result.terminal_id),
            &candidate,
        )
        .unwrap();
        assert_eq!(
            host.creation_runtime_evidence(&binding, planned),
            CreationRuntimeEvidence::Ready
        );
    }
}

#[tokio::test]
async fn noncanonical_and_unrepresentable_runtime_sizes_cannot_finalize_creation() {
    let (_directory, host, request, binding, runtime) = resized_ready_creation_fixture().await;
    let path = host.paths.runtime_record(binding.result.terminal_id);
    let finalize = finalize_request_for(request.creation_token, &binding);
    for (cols, rows) in [
        (u16::default(), runtime.rows),
        (
            crate::model::MIN_TERMINAL_COLUMNS - CREATION_RESIZE_STEP,
            runtime.rows,
        ),
        (runtime.cols, u16::default()),
        (
            runtime.cols,
            crate::model::MIN_TERMINAL_ROWS - CREATION_RESIZE_STEP,
        ),
    ] {
        let mut candidate = runtime.clone();
        candidate.cols = cols;
        candidate.rows = rows;
        atomic_write_json(&path, &candidate).unwrap();
        assert_eq!(
            host.finalize_creation(finalize).await.unwrap_err().code,
            crate::protocol::error_code::IDENTITY_MISMATCH
        );
        assert_resized_creation_still_pending(&host, &binding);
    }
    for field in [crate::protocol::field::COLS, crate::protocol::field::ROWS] {
        let mut encoded = serde_json::to_value(&runtime).unwrap();
        encoded[field] = (u64::from(u16::MAX) + u64::from(CREATION_RESIZE_STEP)).into();
        atomic_write_json(&path, &encoded).unwrap();
        assert_eq!(
            host.finalize_creation(finalize).await.unwrap_err().code,
            crate::protocol::error_code::DELIVERY_UNKNOWN
        );
        assert_resized_creation_still_pending(&host, &binding);
    }
}
