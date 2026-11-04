use super::constants::*;
use yolo_cli_test_support::{CliFixture, FAILURE, SENTINEL, assert_status, flag};
use yolo_release_evidence::*;

#[tokio::test]
async fn read_only_audit_and_verification_dispatch_reject_missing_fixture_inputs() {
    let fixture = CliFixture::new();
    let missing = fixture.path(MISSING_FILE);
    for (operation, argument) in [
        (AUDIT_ARCHIVE_OPERATION, ARCHIVE_ARGUMENT),
        (AUDIT_MANIFEST_OPERATION, MANIFEST_ARGUMENT),
        (AUDIT_BUNDLE_OPERATION, MANIFEST_ARGUMENT),
        (VERIFY_OPERATION, OUTPUT_DIRECTORY_ARGUMENT),
    ] {
        let output = fixture
            .run(
                fixture
                    .command()
                    .arg(operation)
                    .arg(flag(argument))
                    .arg(&missing),
            )
            .await;
        assert_status(&output, FAILURE);
        assert!(output.stdout.is_empty() && !output.stderr.is_empty());
        assert!(!missing.exists());
    }
    let output = fixture
        .run(
            fixture
                .command()
                .arg(VERIFY_ACCEPTANCE_OPERATION)
                .arg(flag(CANDIDATE_DIRECTORY_ARGUMENT))
                .arg(&missing)
                .arg(flag(ACCEPTANCE_RECEIPT_ARGUMENT))
                .arg(&missing),
        )
        .await;
    assert_status(&output, FAILURE);
    assert!(output.stdout.is_empty() && !output.stderr.is_empty());
    assert!(!missing.exists());
}

#[tokio::test]
async fn producing_dispatch_stops_at_local_guards_without_build_signing_or_publication() {
    let fixture = CliFixture::new();
    let sentinel = fixture.path(EXISTING_OUTPUT);
    std::fs::write(&sentinel, SENTINEL).unwrap();
    let missing = fixture.path(MISSING_FILE);
    // OutputTransaction::prepare rejects the existing destination before
    // SystemCandidateExecutor can inspect Git or start Bazel/Docker.
    let output = fixture
        .run(
            fixture
                .command()
                .arg(CANDIDATE_OPERATION)
                .arg(flag(WORKSPACE_ARGUMENT))
                .arg(&missing)
                .arg(flag(OUTPUT_DIRECTORY_ARGUMENT))
                .arg(&sentinel),
        )
        .await;
    assert_status(&output, FAILURE);
    assert_eq!(std::fs::read(&sentinel).unwrap(), SENTINEL);
    let parent = fixture.path(MISSING_PARENT);
    let destination = parent.join(DELIVERY_NAME);
    // DeliveryTarget::prepare cannot open this nonexistent parent, so no stage
    // or lock is created and no extraction/signature process can be reached.
    let output = fixture
        .run(
            fixture
                .command()
                .arg(DELIVER_OPERATION)
                .arg(flag(CANDIDATE_DIRECTORY_ARGUMENT))
                .arg(&missing)
                .arg(flag(OUTPUT_DIRECTORY_ARGUMENT))
                .arg(&destination),
        )
        .await;
    assert_status(&output, FAILURE);
    assert!(!parent.exists());
    // validate_profile runs before target preparation and all external commands.
    let output = fixture
        .run(
            fixture
                .command()
                .arg(DISTRIBUTE_OPERATION)
                .arg(flag(CANDIDATE_DIRECTORY_ARGUMENT))
                .arg(&missing)
                .arg(flag(OUTPUT_DIRECTORY_ARGUMENT))
                .arg(&destination)
                .arg(flag(NOTARY_PROFILE_ARGUMENT))
                .arg(INVALID_NOTARY_PROFILE)
                .arg(flag(ACCEPTANCE_RECEIPT_ARGUMENT))
                .arg(&missing),
        )
        .await;
    assert_status(&output, FAILURE);
    assert!(output.stdout.is_empty() && !output.stderr.is_empty());
    assert!(!parent.exists() && !missing.exists());
    assert_eq!(std::fs::read(sentinel).unwrap(), SENTINEL);
}
