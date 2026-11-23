use crate::developer_release_arguments::*;
use crate::developer_release_command::{
    DeveloperReleaseCommandRunner, SystemDeveloperReleaseCommandRunner, parse_notary_response,
    validate_profile,
};
use crate::developer_release_constants::*;
use std::fs;
use std::sync::Arc;
use yolo_release_contracts::{CODESIGN_DEEP_ARGUMENT, CODESIGN_VERIFY_ARGUMENT};
use yolo_release_filesystem::DirectoryHandle;

#[test]
fn signing_arguments_require_developer_id_runtime_and_timestamp() {
    let arguments = sign_arguments("Yolo.app");
    assert!(arguments.contains(&DEVELOPER_IDENTITY.to_owned()));
    assert!(arguments.contains(&CODESIGN_RUNTIME_OPTION.to_owned()));
    assert!(arguments.contains(&CODESIGN_TIMESTAMP_ARGUMENT.to_owned()));
}

#[test]
fn helper_and_application_verification_use_the_pinned_certificate() {
    let helper = verify_arguments("helper", false);
    let app = verify_arguments("Yolo.app", true);
    assert!(helper.contains(&CODESIGN_VERIFY_ARGUMENT.to_owned()));
    assert!(!helper.contains(&CODESIGN_DEEP_ARGUMENT.to_owned()));
    assert!(app.contains(&CODESIGN_DEEP_ARGUMENT.to_owned()));
    assert!(
        app.iter()
            .any(|argument| argument.contains(DEVELOPER_TEAM_ID))
    );
    assert!(
        app.iter()
            .any(|argument| argument.contains(DEVELOPER_IDENTITY))
    );
}

#[test]
fn notary_submission_is_waited_and_machine_readable() {
    let arguments = notary_arguments("profile");
    assert!(arguments.contains(&PINNED_INPUT_PATH.to_owned()));
    assert!(arguments.contains(&NOTARYTOOL_FORCE_ARGUMENT.to_owned()));
    assert!(arguments.contains(&NOTARYTOOL_WAIT_ARGUMENT.to_owned()));
    assert!(arguments.contains(&NOTARYTOOL_JSON_FORMAT.to_owned()));
    assert!(arguments.contains(&NOTARYTOOL_NO_PROGRESS_ARGUMENT.to_owned()));
}

#[test]
fn accepted_notary_response_returns_its_submission_identifier() {
    let response = br#"{"id":"submission-id","status":"Accepted"}"#;
    assert_eq!(parse_notary_response(response).unwrap(), "submission-id");
}

#[test]
fn malformed_rejected_or_identifier_free_notary_responses_fail_closed() {
    for response in [
        br#"{}"#.as_slice(),
        br#"{"id":"submission-id","status":"Invalid"}"#,
        br#"{"id":"","status":"Accepted"}"#,
    ] {
        assert!(parse_notary_response(response).is_err());
    }
}

#[test]
fn notary_profile_rejects_whitespace_controls_and_oversize_values() {
    assert!(validate_profile("release-profile").is_ok());
    for profile in [
        "",
        " profile",
        "profile ",
        "profile\nname",
        "-profile",
        "profile/name",
    ] {
        assert!(validate_profile(profile).is_err());
    }
    assert!(validate_profile(&"x".repeat(MAX_NOTARY_PROFILE_BYTES + 1)).is_err());
}

#[test]
fn archives_strip_ambient_metadata_and_keep_the_application_parent() {
    let arguments = archive_arguments("Yolo.app", "Yolo.zip");
    for required in [
        DITTO_KEEP_PARENT_ARGUMENT,
        yolo_release_contracts::DITTO_NO_RESOURCE_FORK_ARGUMENT,
        yolo_release_contracts::DITTO_NO_EXTENDED_ATTRIBUTES_ARGUMENT,
        yolo_release_contracts::DITTO_NO_ACL_ARGUMENT,
        yolo_release_contracts::DITTO_NO_QUARANTINE_ARGUMENT,
    ] {
        assert!(arguments.contains(&required.to_owned()));
    }
}

#[test]
fn extraction_reads_the_retained_candidate_descriptor() {
    let arguments = extract_arguments();
    assert!(arguments.contains(&PINNED_INPUT_PATH.to_owned()));
    assert!(!arguments.contains(&yolo_release_contracts::ARCHIVE_NAME.to_owned()));
}

#[test]
fn signature_normalization_uses_the_codesign_remove_operation() {
    assert_eq!(
        remove_signature_arguments("executable"),
        [CODESIGN_REMOVE_SIGNATURE_ARGUMENT, "executable"]
    );
}

#[test]
fn system_ditto_extracts_from_the_inherited_candidate_descriptor() {
    let root = tempfile::tempdir().unwrap();
    let workspace_path = root.path().join("workspace");
    let archive_path = root.path().join("retained-candidate.zip");
    fs::create_dir(&workspace_path).unwrap();
    fs::create_dir(workspace_path.join(yolo_release_contracts::EXTRACTION_DIRECTORY_NAME)).unwrap();
    fs::write(
        &archive_path,
        super::developer_release_fixture_archive::candidate_archive(),
    )
    .unwrap();
    let workspace = DirectoryHandle::open(&workspace_path).unwrap();
    let archive = Arc::new(fs::File::open(archive_path).unwrap());
    SystemDeveloperReleaseCommandRunner
        .extract(&workspace, archive)
        .unwrap();
    assert!(
        workspace_path
            .join(yolo_release_contracts::EXTRACTION_DIRECTORY_NAME)
            .join(yolo_release_contracts::APP_NAME)
            .is_dir()
    );
}
