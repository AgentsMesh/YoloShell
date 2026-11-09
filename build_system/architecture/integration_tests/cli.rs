use std::path::PathBuf;
use yolo_architecture::{TEST_SOURCE_DIRECTORY_ENV, TEST_WORKSPACE_ENV};
use yolo_architecture_source_policy::constants::MAX_HANDWRITTEN_LINES;
use yolo_architecture_test_support::cli_constants::*;
use yolo_cli_test_support::{CliFixture, SUCCESS, assert_status};

fn roots(fixture: &CliFixture) -> (PathBuf, PathBuf, String) {
    let runfiles = fixture.path(RUNFILES_DIRECTORY);
    let workspace = std::env::var(TEST_WORKSPACE_ENV).unwrap();
    let root = runfiles.join(&workspace);
    std::fs::create_dir_all(root.join(SOURCE_FILE).parent().unwrap()).unwrap();
    std::fs::write(root.join(SOURCE_FILE), VALID_SOURCE).unwrap();
    (runfiles, root, workspace)
}

#[tokio::test]
async fn actual_checker_reports_success_for_a_valid_owned_source_tree() {
    let fixture = CliFixture::new();
    let (runfiles, root, workspace) = roots(&fixture);
    let output = fixture
        .run(
            fixture
                .command()
                .arg(&root)
                .env(TEST_SOURCE_DIRECTORY_ENV, runfiles)
                .env(TEST_WORKSPACE_ENV, workspace),
        )
        .await;
    assert_status(&output, SUCCESS);
    assert!(output.stderr.is_empty());
    assert!(String::from_utf8_lossy(&output.stdout).contains(PASSED_MESSAGE));
    assert_eq!(
        std::fs::read_to_string(root.join(SOURCE_FILE)).unwrap(),
        VALID_SOURCE
    );
}

#[tokio::test]
async fn actual_checker_reports_violations_without_modifying_the_fixture() {
    let fixture = CliFixture::new();
    let (runfiles, root, workspace) = roots(&fixture);
    let oversized = SOURCE_LINE.repeat(MAX_HANDWRITTEN_LINES + EXCESS_LINE);
    std::fs::write(root.join(OVERSIZED_FILE), &oversized).unwrap();
    let output = fixture
        .run(
            fixture
                .command()
                .arg(&root)
                .env(TEST_SOURCE_DIRECTORY_ENV, runfiles)
                .env(TEST_WORKSPACE_ENV, workspace),
        )
        .await;
    assert_status(&output, POLICY_FAILURE);
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains(LIMIT_MESSAGE));
    assert_eq!(
        std::fs::read_to_string(root.join(OVERSIZED_FILE)).unwrap(),
        oversized
    );
}

#[tokio::test]
async fn actual_checker_rejects_missing_root_instead_of_falling_back_to_user_workspace() {
    let fixture = CliFixture::new();
    let (runfiles, _, workspace) = roots(&fixture);
    let missing = fixture.path(MISSING_DIRECTORY);
    let output = fixture
        .run(
            fixture
                .command()
                .arg(&missing)
                .env(TEST_SOURCE_DIRECTORY_ENV, runfiles)
                .env(TEST_WORKSPACE_ENV, workspace),
        )
        .await;
    assert_status(&output, CHECK_FAILURE);
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains(FAILED_MESSAGE));
    assert!(!missing.exists());
}
