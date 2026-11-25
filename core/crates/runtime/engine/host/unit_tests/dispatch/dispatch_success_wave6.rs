use crate::host::tests::*;
const DISPATCH_WAVE6_ARCHIVE_NAME: &str = "dispatch-archive-wave6";
const DISPATCH_WAVE6_TERMINATE_NAME: &str = "dispatch-terminate-wave6";
const DISPATCH_WAVE6_FINALIZE_NAME: &str = "dispatch-finalize-wave6";

#[tokio::test]
async fn dispatch_success_paths_encode_catalog_and_creation_results() {
    let directory = tempdir().unwrap();
    let manifest = persist_rebuild_manifest(directory.path());
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    for response in [
        host.dispatch(&request(
            method::GET_SESSION,
            &TerminalRef {
                terminal_id: manifest.terminal_id,
            },
        ))
        .await,
        host.dispatch(&request(
            method::ARCHIVE_SESSION,
            &TerminalLifecycleSelection::unconditional(manifest.terminal_id),
        ))
        .await,
        host.dispatch(&request(
            method::DISCARD_SESSION,
            &TerminalDiscardRequest {
                selection: TerminalLifecycleSelection::unconditional(manifest.terminal_id),
                force: false,
            },
        ))
        .await,
        host.dispatch(&request(method::RECONCILE, &())).await,
    ] {
        assert!(response.ok, "dispatch failed: {:?}", response.error);
    }

    let (_archive_directory, archive_host, _, archive_binding, _, _, _) =
        lifecycle_product_fixture(DISPATCH_WAVE6_ARCHIVE_NAME, CreationPhase::Terminated).await;
    let archive_request = CreationCleanupArchiveRequest {
        terminal_id: archive_binding.result.terminal_id,
        creation_capability: archive_binding.result.creation_capability,
    };
    assert!(
        archive_host
            .dispatch(&request(method::CLEANUP_ARCHIVE_SESSION, &archive_request))
            .await
            .ok
    );
    assert!(
        archive_host
            .dispatch(&request(
                method::CLEANUP_DISCARD_SESSION,
                &CreationCleanupDiscardRequest {
                    terminal_id: archive_request.terminal_id,
                    creation_capability: archive_request.creation_capability,
                    force: false,
                },
            ))
            .await
            .ok
    );

    let (_terminate_directory, terminate_host, _, terminate_binding, _, _, _) =
        lifecycle_product_fixture(DISPATCH_WAVE6_TERMINATE_NAME, CreationPhase::Ready).await;
    assert!(
        terminate_host
            .dispatch(&request(
                method::CLEANUP_TERMINATE_SESSION,
                &CreationCleanupTerminateRequest {
                    terminal_id: terminate_binding.result.terminal_id,
                    creation_capability: terminate_binding.result.creation_capability,
                    force: false,
                },
            ))
            .await
            .ok
    );

    let (_finalize_directory, finalize_host, create_request, finalize_binding, _, _, _) =
        lifecycle_product_fixture(DISPATCH_WAVE6_FINALIZE_NAME, CreationPhase::Ready).await;
    assert!(
        finalize_host
            .dispatch(&request(
                method::FINALIZE_CREATION,
                &FinalizeCreationRequest {
                    creation_token: create_request.creation_token,
                    terminal_id: finalize_binding.result.terminal_id,
                    creation_capability: finalize_binding.result.creation_capability,
                },
            ))
            .await
            .ok
    );
}
