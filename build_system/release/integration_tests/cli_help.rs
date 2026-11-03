use super::constants::*;
use yolo_cli_test_support::{
    CliFixture, HELP_ARGUMENT, USAGE_FAILURE, USAGE_MARKER, assert_status, flag, successful_text,
};
use yolo_release_evidence::*;

#[tokio::test]
async fn help_preserves_public_contracts_without_running_release_operations() {
    let fixture = CliFixture::new();
    let root = fixture.args(&[HELP_ARGUMENT]).await;
    assert!(successful_text(&root).contains(USAGE_MARKER));
    let operations: &[(&str, &[&str], bool)] = &[
        (
            CANDIDATE_OPERATION,
            &[WORKSPACE_ARGUMENT, OUTPUT_DIRECTORY_ARGUMENT],
            true,
        ),
        (
            DELIVER_OPERATION,
            &[CANDIDATE_DIRECTORY_ARGUMENT, OUTPUT_DIRECTORY_ARGUMENT],
            true,
        ),
        (
            DISTRIBUTE_OPERATION,
            &[
                CANDIDATE_DIRECTORY_ARGUMENT,
                OUTPUT_DIRECTORY_ARGUMENT,
                NOTARY_PROFILE_ARGUMENT,
                ACCEPTANCE_RECEIPT_ARGUMENT,
            ],
            true,
        ),
        (VERIFY_OPERATION, &[OUTPUT_DIRECTORY_ARGUMENT], true),
        (
            VERIFY_ACCEPTANCE_OPERATION,
            &[CANDIDATE_DIRECTORY_ARGUMENT, ACCEPTANCE_RECEIPT_ARGUMENT],
            true,
        ),
        (AUDIT_ARCHIVE_OPERATION, &[ARCHIVE_ARGUMENT], false),
        (AUDIT_MANIFEST_OPERATION, &[MANIFEST_ARGUMENT], false),
        (AUDIT_BUNDLE_OPERATION, &[MANIFEST_ARGUMENT], false),
    ];
    for (operation, required, rejects_retired) in operations {
        let output = fixture.args(&[*operation, HELP_ARGUMENT]).await;
        let help = successful_text(&output);
        for argument in *required {
            assert!(help.contains(&flag(argument)), "{operation}: {argument}");
        }
        if *rejects_retired {
            for argument in RETIRED_ARGUMENTS {
                assert!(
                    !help.contains(&flag(argument)),
                    "{operation} exposes retired {argument}"
                );
            }
        }
        if *operation == VERIFY_ACCEPTANCE_OPERATION {
            for argument in [OUTPUT_DIRECTORY_ARGUMENT, NOTARY_PROFILE_ARGUMENT] {
                assert!(!help.contains(&flag(argument)));
            }
        }
    }
}

#[tokio::test]
async fn retired_operations_and_caller_selected_auditor_are_rejected_before_dispatch() {
    let fixture = CliFixture::new();
    for operation in RETIRED_OPERATIONS {
        let output = fixture.args(&[*operation]).await;
        assert_status(&output, USAGE_FAILURE);
        assert!(output.stdout.is_empty());
    }
    let auditor = fixture.path(FAKE_AUDIT_TOOL);
    let output = fixture
        .run(
            fixture
                .command()
                .arg(CANDIDATE_OPERATION)
                .arg(flag(AUDIT_TOOL_ARGUMENT))
                .arg(&auditor)
                .arg(flag(WORKSPACE_ARGUMENT))
                .arg(CURRENT_DIRECTORY)
                .arg(flag(OUTPUT_DIRECTORY_ARGUMENT))
                .arg(CURRENT_DIRECTORY),
        )
        .await;
    assert_status(&output, USAGE_FAILURE);
    assert!(output.stdout.is_empty());
    assert!(!auditor.exists());
}
