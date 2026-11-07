use crate::developer_release_constants::*;
use yolo_release_contracts::{
    APP_NAME, CODESIGN_ARGUMENTS, DITTO_ARGUMENTS, EXTRACTION_DIRECTORY_NAME,
};

pub fn signing_paths(extracted: bool) -> Vec<String> {
    SIGNED_ARCHIVE_PATHS
        .iter()
        .map(|path| workspace_path(path, extracted))
        .collect()
}

pub fn app_path(extracted: bool) -> String {
    workspace_path(APP_ARCHIVE_PATH, extracted)
}

pub fn sign_arguments(path: &str) -> Vec<String> {
    [
        CODESIGN_FORCE_ARGUMENT,
        CODESIGN_SIGN_ARGUMENT,
        DEVELOPER_IDENTITY,
        CODESIGN_OPTIONS_ARGUMENT,
        CODESIGN_RUNTIME_OPTION,
        CODESIGN_TIMESTAMP_ARGUMENT,
        CODESIGN_VERBOSE_ARGUMENT,
        path,
    ]
    .into_iter()
    .map(str::to_owned)
    .collect()
}

pub fn verify_arguments(path: &str, deep: bool) -> Vec<String> {
    let mut arguments = CODESIGN_ARGUMENTS
        .iter()
        .copied()
        .chain([CODESIGN_VERBOSE_ARGUMENT])
        .map(str::to_owned)
        .collect::<Vec<_>>();
    if !deep {
        arguments.retain(|argument| argument != yolo_release_contracts::CODESIGN_DEEP_ARGUMENT);
    }
    arguments.push(format!(
        "{CODESIGN_REQUIREMENT_PREFIX}{DEVELOPER_REQUIREMENT_TEAM_PREFIX}{DEVELOPER_TEAM_ID}{DEVELOPER_REQUIREMENT_COMMON_NAME_PREFIX}{DEVELOPER_IDENTITY}\""
    ));
    arguments.push(path.to_owned());
    arguments
}

pub fn extract_arguments() -> Vec<String> {
    DITTO_ARGUMENTS
        .iter()
        .copied()
        .chain([PINNED_INPUT_PATH, EXTRACTION_DIRECTORY_NAME])
        .map(str::to_owned)
        .collect()
}

pub fn remove_signature_arguments(path: &str) -> Vec<String> {
    [CODESIGN_REMOVE_SIGNATURE_ARGUMENT, path]
        .into_iter()
        .map(str::to_owned)
        .collect()
}

pub fn archive_arguments(source: &str, destination: &str) -> Vec<String> {
    [
        DITTO_CREATE_ARGUMENT,
        DITTO_ZIP_ARGUMENT,
        DITTO_KEEP_PARENT_ARGUMENT,
        yolo_release_contracts::DITTO_NO_RESOURCE_FORK_ARGUMENT,
        yolo_release_contracts::DITTO_NO_EXTENDED_ATTRIBUTES_ARGUMENT,
        yolo_release_contracts::DITTO_NO_ACL_ARGUMENT,
        yolo_release_contracts::DITTO_NO_QUARANTINE_ARGUMENT,
        source,
        destination,
    ]
    .into_iter()
    .map(str::to_owned)
    .collect()
}

pub fn notary_arguments(profile: &str) -> Vec<String> {
    [
        NOTARYTOOL_SUBCOMMAND,
        NOTARYTOOL_SUBMIT_SUBCOMMAND,
        PINNED_INPUT_PATH,
        NOTARYTOOL_FORCE_ARGUMENT,
        NOTARYTOOL_KEYCHAIN_PROFILE_ARGUMENT,
        profile,
        NOTARYTOOL_WAIT_ARGUMENT,
        NOTARYTOOL_TIMEOUT_ARGUMENT,
        NOTARYTOOL_TIMEOUT_VALUE,
        NOTARYTOOL_OUTPUT_FORMAT_ARGUMENT,
        NOTARYTOOL_JSON_FORMAT,
        NOTARYTOOL_NO_PROGRESS_ARGUMENT,
    ]
    .into_iter()
    .map(str::to_owned)
    .collect()
}

pub fn stapler_arguments(operation: &str, path: &str) -> Vec<String> {
    [
        STAPLER_SUBCOMMAND,
        operation,
        STAPLER_VERBOSE_ARGUMENT,
        path,
    ]
    .into_iter()
    .map(str::to_owned)
    .collect()
}

pub fn assessment_arguments(path: &str) -> Vec<String> {
    [
        SPCTL_ASSESS_ARGUMENT,
        SPCTL_TYPE_ARGUMENT,
        SPCTL_EXECUTE_TYPE,
        SPCTL_VERBOSE_ARGUMENT,
        path,
    ]
    .into_iter()
    .map(str::to_owned)
    .collect()
}

fn workspace_path(archive_path: &str, extracted: bool) -> String {
    if extracted {
        format!("{EXTRACTION_DIRECTORY_NAME}/{archive_path}")
    } else if archive_path == APP_ARCHIVE_PATH {
        APP_NAME.to_owned()
    } else {
        archive_path.to_owned()
    }
}
