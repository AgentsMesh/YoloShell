const CTL_LIFECYCLE_UPDATED_AT: &str = "13";
const CTL_LIFECYCLE_RUNNING_STATUS: &str = "running";
const CTL_LIFECYCLE_MANIFEST_SHA256: &str =
    "b6c4f71b4cfb9a3dad44f196d9939984013a1a9c4883c7ce2cbde8e6382598cc";

#[tokio::test]
async fn ctl_recreate_carries_the_exact_catalog_selection() {
    run_ctl_remote_failure(&[
        cli::RESUME_COMMAND,
        SESSION_ID,
        cli::RECREATE_ARGUMENT,
        cli::EXPECTED_UPDATED_AT_ARGUMENT,
        CTL_LIFECYCLE_UPDATED_AT,
        cli::EXPECTED_STATUS_ARGUMENT,
        CTL_LIFECYCLE_RUNNING_STATUS,
        cli::EXPECTED_MANIFEST_SHA256_ARGUMENT,
        CTL_LIFECYCLE_MANIFEST_SHA256,
        cli::EXPECTED_RUNTIME_EPOCH_ARGUMENT,
        SESSION_ID,
    ])
    .await;
}

#[tokio::test]
async fn ctl_conditional_lifecycle_reaches_archive_and_discard_requests() {
    for command in [cli::ARCHIVE_COMMAND, cli::DISCARD_COMMAND] {
        let mut arguments = vec![command, SESSION_ID];
        if command == cli::DISCARD_COMMAND {
            arguments.push(cli::FORCE_ARGUMENT);
        }
        arguments.extend([
            cli::EXPECTED_UPDATED_AT_ARGUMENT,
            CTL_LIFECYCLE_UPDATED_AT,
            cli::EXPECTED_STATUS_ARGUMENT,
            CTL_LIFECYCLE_RUNNING_STATUS,
        ]);
        run_ctl_remote_failure(&arguments).await;
    }
}

#[tokio::test]
async fn ctl_runtime_epoch_fence_is_available_to_each_product_lifecycle_command() {
    for command in [
        cli::TERMINATE_COMMAND,
        cli::ARCHIVE_COMMAND,
        cli::DISCARD_COMMAND,
    ] {
        run_ctl_remote_failure(&[
            command,
            SESSION_ID,
            cli::EXPECTED_RUNTIME_EPOCH_ARGUMENT,
            SESSION_ID,
        ])
        .await;
    }
}

#[tokio::test]
async fn ctl_manifest_digest_fence_is_available_to_each_product_lifecycle_command() {
    for command in [
        cli::TERMINATE_COMMAND,
        cli::ARCHIVE_COMMAND,
        cli::DISCARD_COMMAND,
    ] {
        run_ctl_remote_failure(&[
            command,
            SESSION_ID,
            cli::EXPECTED_MANIFEST_SHA256_ARGUMENT,
            CTL_LIFECYCLE_MANIFEST_SHA256,
        ])
        .await;
    }
}

#[test]
fn ctl_conditional_lifecycle_requires_a_complete_precondition() {
    let root = tempfile::tempdir().unwrap();
    let socket = root.path().join(CTL_MATRIX_SOCKET_NAME);
    let prefix = [
        yolo_launch_contract::EXECUTABLE_NAME,
        cli::CONTROL_COMMAND,
        cli::SOCKET_ARGUMENT,
        socket.to_str().unwrap(),
        cli::EXISTING_SOCKET_ARGUMENT,
        cli::ARCHIVE_COMMAND,
        SESSION_ID,
    ];
    assert!(
        Cli::try_parse_from(
            prefix
                .into_iter()
                .chain([cli::EXPECTED_UPDATED_AT_ARGUMENT, CTL_LIFECYCLE_UPDATED_AT,])
        )
        .is_err()
    );
    assert!(
        Cli::try_parse_from(
            prefix
                .into_iter()
                .chain([cli::EXPECTED_STATUS_ARGUMENT, CTL_LIFECYCLE_RUNNING_STATUS,])
        )
        .is_err()
    );
}
