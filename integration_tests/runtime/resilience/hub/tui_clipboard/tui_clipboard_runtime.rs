use super::{Driver, constants::*, deadline};
use crate::{Harness, runtime_ref_from_view};
use yolo_hub_identity::RuntimeTarget;
use yolo_runtime::model::{TerminalLifecycleSelection, TerminalManifestDigest, TerminalRuntimeStatus};

pub async fn replace(host: &Harness, driver: &mut Driver, previous: RuntimeTarget) {
    let client = host.client();
    let session = previous.address.terminal_id.as_uuid();
    client.terminate(session, false).await.unwrap();
    let selection = exited_selection(host, previous).await;
    let recreated = host
        .resume_with_preconditions_and_track(selection, true)
        .await
        .unwrap();
    let runtime = runtime_ref_from_view(&recreated);
    assert_ne!(runtime.runtime_epoch, previous.runtime_epoch.as_uuid());
    let grant = client
        .acquire_writer(
            runtime,
            REPLACEMENT_WRITER,
            Some(yolo_runtime::lease::constants::MAX_TTL_MS),
        )
        .await
        .unwrap();
    client
        .input(
            runtime,
            grant.generation,
            &grant.writer_capability,
            REPLACEMENT_COMMAND.to_vec(),
        )
        .await
        .unwrap();
    // New-epoch output proves the TUI has replaced its observed runtime.
    driver.wait(|text| text.contains(REPLACEMENT_MARKER)).await;
    driver.send(COPY).await;
    driver.wait(|text| text.contains(NO_SELECTION_LABEL)).await;
    client.release_writer(&grant, runtime).await.unwrap();
}

async fn exited_selection(host: &Harness, previous: RuntimeTarget) -> TerminalLifecycleSelection {
    let client = host.client();
    let session = previous.address.terminal_id.as_uuid();
    tokio::time::timeout_at(deadline(), async {
        loop {
            let view = client.get_session(session).await.unwrap();
            let runtime = view.runtime.as_ref().expect(RETIRED_RUNTIME_EXPECTED);
            assert_eq!(runtime.runtime_epoch, previous.runtime_epoch.as_uuid());
            if runtime.status == TerminalRuntimeStatus::Exited {
                assert!(!view.manifest.archived);
                assert!(view.agent.is_none());
                return TerminalLifecycleSelection::selected(
                    view.manifest.terminal_id,
                    view.manifest.updated_at_ms,
                    runtime.status,
                )
                .with_expected_manifest_sha256(Some(
                    TerminalManifestDigest::capture(&view.manifest).unwrap(),
                ))
                .with_expected_runtime_epoch(Some(runtime.runtime_epoch));
            }
            tokio::time::sleep(yolo_terminal_query_fixture::constants::POLL).await;
        }
    })
    .await
    .expect(RETIRED_RUNTIME_DEADLINE)
}
