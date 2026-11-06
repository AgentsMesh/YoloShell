use crate::host::tests::*;
use super::*;

#[derive(Clone, Copy)]
enum ProductLifecycleQueueOrder {
    FinalizeFirst,
    ArchiveFirst,
}

async fn queue_finalize_and_archive(
    host: &Host,
    request: FinalizeCreationRequest,
    order: ProductLifecycleQueueOrder,
) -> (
    std::result::Result<(), ErrorBody>,
    std::result::Result<TerminalView, ErrorBody>,
) {
    let terminal_id = request.terminal_id;
    let guard = host.session_locks.lock(terminal_id).await;
    let mut finalize = Box::pin(host.finalize_creation(request));
    let mut archive = Box::pin(host.archive_session(TerminalRef { terminal_id }));
    match order {
        ProductLifecycleQueueOrder::FinalizeFirst => {
            assert!(matches!(
                futures::poll!(&mut finalize),
                std::task::Poll::Pending
            ));
            assert!(matches!(
                futures::poll!(&mut archive),
                std::task::Poll::Pending
            ));
        }
        ProductLifecycleQueueOrder::ArchiveFirst => {
            assert!(matches!(
                futures::poll!(&mut archive),
                std::task::Poll::Pending
            ));
            assert!(matches!(
                futures::poll!(&mut finalize),
                std::task::Poll::Pending
            ));
        }
    }
    drop(guard);
    tokio::time::timeout(HOST_BLOCKING_TEST_TIMEOUT, async {
        tokio::join!(finalize, archive)
    })
    .await
    .unwrap()
}

fn product_finalize_request(
    request: &CreateTerminal,
    binding: &CreationBinding,
) -> FinalizeCreationRequest {
    FinalizeCreationRequest {
        creation_token: request.creation_token,
        terminal_id: binding.result.terminal_id,
        creation_capability: binding.result.creation_capability,
    }
}

fn assert_product_completion(host: &Host, binding: &CreationBinding, expected: CreationCompletion) {
    assert_eq!(
        host.creation_receipts
            .lookup_binding(binding)
            .unwrap()
            .unwrap()
            .completion,
        expected
    );
}

#[tokio::test]
async fn finalize_then_ordinary_archive_serializes_and_replays_finalized_receipt() {
    let (_directory, host, request, binding, _manifest, _runtime, path) =
        lifecycle_product_fixture(PRODUCT_ARCHIVE_BARRIER_NAME, CreationPhase::Ready).await;
    let finalize_request = product_finalize_request(&request, &binding);
    let (finalized, archived) = queue_finalize_and_archive(
        &host,
        finalize_request,
        ProductLifecycleQueueOrder::FinalizeFirst,
    )
    .await;
    finalized.unwrap();
    assert!(archived.unwrap().manifest.archived);
    assert_transition_ledger(&path, &[]);
    assert_product_completion(&host, &binding, CreationCompletion::Finalized);
    host.finalize_creation(finalize_request).await.unwrap();
    assert!(
        host.archive_session(TerminalRef {
            terminal_id: binding.result.terminal_id,
        })
        .await
        .unwrap()
        .manifest
        .archived
    );
    assert_product_completion(&host, &binding, CreationCompletion::Finalized);
}

#[tokio::test]
async fn ordinary_archive_then_finalize_serializes_and_replays_archived_receipt() {
    let (_directory, host, request, binding, _manifest, _runtime, path) =
        lifecycle_product_fixture(PRODUCT_ARCHIVE_BARRIER_NAME, CreationPhase::Ready).await;
    let finalize_request = product_finalize_request(&request, &binding);
    let (finalized, archived) = queue_finalize_and_archive(
        &host,
        finalize_request,
        ProductLifecycleQueueOrder::ArchiveFirst,
    )
    .await;
    assert!(archived.unwrap().manifest.archived);
    assert_eq!(
        finalized.unwrap_err().code,
        crate::protocol::error_code::UNAUTHORIZED
    );
    assert_transition_ledger(&path, &[]);
    assert_product_completion(&host, &binding, CreationCompletion::Archived);
    assert!(
        host.archive_session(TerminalRef {
            terminal_id: binding.result.terminal_id,
        })
        .await
        .unwrap()
        .manifest
        .archived
    );
    assert_eq!(
        host.finalize_creation(finalize_request)
            .await
            .unwrap_err()
            .code,
        crate::protocol::error_code::UNAUTHORIZED
    );
    assert_product_completion(&host, &binding, CreationCompletion::Archived);
}
