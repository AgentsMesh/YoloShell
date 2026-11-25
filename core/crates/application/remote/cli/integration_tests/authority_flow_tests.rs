use super::*;
use std::os::unix::fs::PermissionsExt;

#[tokio::test]
async fn serve_status_start_and_retire_cover_detached_management_flow() {
    let (fixture, task) = running_fixture().await;
    let paths = BrokerPaths::new(&fixture.options.broker_state).unwrap();
    let mutation_id = uuid::Uuid::new_v4();
    let authority_root = fixture.options.config_source.parent().unwrap();
    let authority_document = authority_root.join(format!(
        "{}{}",
        fixture::TEST_HOST_ID,
        fixture::AUTHORITY_DOCUMENT_SUFFIX
    ));
    let authority_lock = authority_root.join(format!(
        "{}{}",
        fixture::TEST_HOST_ID,
        fixture::AUTHORITY_LOCK_SUFFIX
    ));
    write_committed(&authority_document);
    std::fs::write(&authority_lock, fixture::AUTHORITY_LOCK_MARKER).unwrap();
    for path in [&authority_document, &authority_lock] {
        std::fs::set_permissions(
            path,
            std::fs::Permissions::from_mode(fixture::PRIVATE_FILE_MODE),
        )
        .unwrap();
    }
    run(Cli {
        command: Command::Status {
            socket: paths.management_socket(),
        },
    })
    .await
    .unwrap();
    let mut start = server_options::from_request(&fixture.options);
    start.authority_document = Some(authority_document.clone());
    start.authority_lock = Some(authority_lock.clone());
    start.expected_profile_sha256 = Some(fixture::TEST_PROFILE_AUTHORITY_SHA256.into());
    run(Cli {
        command: Command::Start(start),
    })
    .await
    .unwrap();
    write_blocked(&authority_document, mutation_id);
    run(Cli {
        command: Command::Retire(RetireOptions {
            state_dir: fixture.options.broker_state.clone(),
            expected_host: fixture::TEST_HOST_ID.into(),
            authority_document,
            authority_lock,
            expected_mutation_id: mutation_id,
        }),
    })
    .await
    .unwrap();
    task.await.unwrap().unwrap();
}

fn write_committed(path: &std::path::Path) {
    std::fs::write(
        path,
        serde_json::to_vec(&serde_json::json!({
            "version": 1,
            "hostID": fixture::TEST_HOST_ID,
            "phase": "committed",
            "profile": { "hostID": fixture::TEST_HOST_ID },
            "profileSHA256": fixture::TEST_PROFILE_AUTHORITY_SHA256,
        }))
        .unwrap(),
    )
    .unwrap();
}

fn write_blocked(path: &std::path::Path, mutation_id: uuid::Uuid) {
    std::fs::write(
        path,
        serde_json::to_vec(&serde_json::json!({
            "version": 1,
            "hostID": fixture::TEST_HOST_ID,
            "phase": "blocked",
            "mutationID": mutation_id,
        }))
        .unwrap(),
    )
    .unwrap();
}
