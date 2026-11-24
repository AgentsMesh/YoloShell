use crate::host::tests::*;
use super::*;

#[tokio::test]
async fn runtime_resize_does_not_allow_manifest_or_launch_geometry_to_drift() {
    for (manifest, columns) in [(false, false), (false, true), (true, false), (true, true)] {
        let (_directory, host, request, binding, _) = resized_ready_creation_fixture().await;
        let session = binding.result.terminal_id;
        if manifest {
            let path = host.paths.manifest(session);
            let mut value: TerminalManifest = read_json(&path).unwrap();
            if columns {
                value.cols += CREATION_RESIZE_STEP;
            } else {
                value.rows += CREATION_RESIZE_STEP;
            }
            atomic_write_json(&path, &value).unwrap();
        } else {
            let path = host.paths.worker_launch(session);
            let mut value: WorkerLaunchSpec = read_json(&path).unwrap();
            if columns {
                value.cols += CREATION_RESIZE_STEP;
            } else {
                value.rows += CREATION_RESIZE_STEP;
            }
            atomic_write_json(&path, &value).unwrap();
        }
        let finalize = finalize_request_for(request.creation_token, &binding);
        assert_eq!(
            host.finalize_creation(finalize).await.unwrap_err().code,
            crate::protocol::error_code::IDENTITY_MISMATCH
        );
        assert_resized_creation_still_pending(&host, &binding);
    }
}

#[tokio::test]
async fn resized_runtime_still_rejects_foreign_session_epoch_worker_and_authentication() {
    let (_directory, host, request, binding, runtime) = resized_ready_creation_fixture().await;
    let mutations: &[fn(&mut RuntimeRecord)] = &[
        |record| record.terminal_id = Uuid::new_v4(),
        |record| record.runtime_epoch = Uuid::new_v4(),
        |record| record.worker_id = Uuid::new_v4(),
        |record| record.auth_token = CreationLaunchIdentity::new().auth_token,
        |record| record.socket_path.push(CREATION_MISMATCH_PATH),
        |record| record.boot_id.clear(),
        |record| record.worker_pid = record.child_pid,
    ];
    for mutate in mutations {
        let mut candidate = runtime.clone();
        mutate(&mut candidate);
        atomic_write_json(
            &host.paths.runtime_record(binding.result.terminal_id),
            &candidate,
        )
        .unwrap();
        let finalize = finalize_request_for(request.creation_token, &binding);
        assert_eq!(
            host.finalize_creation(finalize).await.unwrap_err().code,
            crate::protocol::error_code::IDENTITY_MISMATCH
        );
        assert_resized_creation_still_pending(&host, &binding);
    }
}

#[tokio::test]
async fn valid_runtime_resize_cannot_authorize_another_creation_token_or_capability() {
    let (_directory, host, request, binding, _) = resized_ready_creation_fixture().await;
    let mutations: &[fn(&mut FinalizeCreationRequest)] = &[
        |request| request.creation_token = Uuid::new_v4(),
        |request| request.terminal_id = Uuid::new_v4(),
        |request| request.creation_capability = Uuid::new_v4(),
    ];
    for mutate in mutations {
        let mut finalize = finalize_request_for(request.creation_token, &binding);
        mutate(&mut finalize);
        assert_eq!(
            host.finalize_creation(finalize).await.unwrap_err().code,
            crate::protocol::error_code::UNAUTHORIZED
        );
        assert_resized_creation_still_pending(&host, &binding);
    }
}
