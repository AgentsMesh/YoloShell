use yolo_release_contracts::{
    APP_NAME, CODESIGN_DEEP_ARGUMENT, CODESIGN_STRICT_ARGUMENT, CODESIGN_VERIFY_ARGUMENT,
    DITTO_ARGUMENTS, DITTO_NO_ACL_ARGUMENT, DITTO_NO_EXTENDED_ATTRIBUTES_ARGUMENT,
    DITTO_NO_QUARANTINE_ARGUMENT, DITTO_NO_RESOURCE_FORK_ARGUMENT, EXTRACTION_DIRECTORY_NAME,
};
use yolo_release_developer_release_system::developer_release_arguments as arguments;
use yolo_release_developer_release_system::developer_release_constants::*;

const PATH_WITH_SPACES: &str = "isolated directory/application";
const ARCHIVE_WITH_SPACES: &str = "isolated directory/archive.zip";
const PROFILE_WITH_SPACES: &str = "release profile";
const RELEASE_EXECUTABLES: &[&str] = &[
    "Yolo.app/Contents/MacOS/Yolo",
    "Yolo.app/Contents/Helpers/yolo",
];

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

#[test]
fn signing_paths_cover_only_the_pinned_executables_in_both_tree_layouts() {
    assert_eq!(
        arguments::signing_paths(false),
        strings(SIGNED_ARCHIVE_PATHS)
    );
    let extracted = SIGNED_ARCHIVE_PATHS
        .iter()
        .map(|path| format!("{EXTRACTION_DIRECTORY_NAME}/{path}"))
        .collect::<Vec<_>>();
    assert_eq!(arguments::signing_paths(true), extracted);
    assert_eq!(arguments::app_path(false), APP_NAME);
    assert_eq!(
        arguments::app_path(true),
        format!("{EXTRACTION_DIRECTORY_NAME}/{APP_ARCHIVE_PATH}")
    );
}

#[test]
fn native_application_and_one_runtime_are_the_exact_signing_targets() {
    let paths = arguments::signing_paths(false);
    assert_eq!(paths, strings(RELEASE_EXECUTABLES));
    let unique = paths.iter().collect::<std::collections::BTreeSet<_>>();
    assert_eq!(unique.len(), paths.len());
}

#[test]
fn signatures_pin_identity_hardened_runtime_and_timestamp_without_splitting_paths() {
    assert_eq!(
        arguments::sign_arguments(PATH_WITH_SPACES),
        strings(&[
            CODESIGN_FORCE_ARGUMENT,
            CODESIGN_SIGN_ARGUMENT,
            DEVELOPER_IDENTITY,
            CODESIGN_OPTIONS_ARGUMENT,
            CODESIGN_RUNTIME_OPTION,
            CODESIGN_TIMESTAMP_ARGUMENT,
            CODESIGN_VERBOSE_ARGUMENT,
            PATH_WITH_SPACES,
        ])
    );
    assert_eq!(
        arguments::remove_signature_arguments(PATH_WITH_SPACES),
        strings(&[CODESIGN_REMOVE_SIGNATURE_ARGUMENT, PATH_WITH_SPACES,])
    );
}

#[test]
fn signature_verification_pins_both_certificate_fields_and_outer_app_depth() {
    let requirement = format!(
        "{CODESIGN_REQUIREMENT_PREFIX}{DEVELOPER_REQUIREMENT_TEAM_PREFIX}{DEVELOPER_TEAM_ID}{DEVELOPER_REQUIREMENT_COMMON_NAME_PREFIX}{DEVELOPER_IDENTITY}\""
    );
    for deep in [false, true] {
        let mut expected = strings(&[CODESIGN_VERIFY_ARGUMENT]);
        if deep {
            expected.push(CODESIGN_DEEP_ARGUMENT.to_owned());
        }
        expected.extend(strings(&[
            CODESIGN_STRICT_ARGUMENT,
            CODESIGN_VERBOSE_ARGUMENT,
        ]));
        expected.extend([requirement.clone(), PATH_WITH_SPACES.to_owned()]);
        assert_eq!(
            arguments::verify_arguments(PATH_WITH_SPACES, deep),
            expected
        );
    }
}

#[test]
fn extraction_uses_the_retained_descriptor_and_archive_creation_strips_ambient_metadata() {
    let mut extraction = strings(DITTO_ARGUMENTS);
    extraction.extend(strings(&[PINNED_INPUT_PATH, EXTRACTION_DIRECTORY_NAME]));
    assert_eq!(arguments::extract_arguments(), extraction);
    assert_eq!(
        arguments::archive_arguments(PATH_WITH_SPACES, ARCHIVE_WITH_SPACES),
        strings(&[
            DITTO_CREATE_ARGUMENT,
            DITTO_ZIP_ARGUMENT,
            DITTO_KEEP_PARENT_ARGUMENT,
            DITTO_NO_RESOURCE_FORK_ARGUMENT,
            DITTO_NO_EXTENDED_ATTRIBUTES_ARGUMENT,
            DITTO_NO_ACL_ARGUMENT,
            DITTO_NO_QUARANTINE_ARGUMENT,
            PATH_WITH_SPACES,
            ARCHIVE_WITH_SPACES,
        ])
    );
}

#[test]
fn notarization_keeps_profile_as_one_argument_and_requires_bounded_json_completion() {
    assert_eq!(
        arguments::notary_arguments(PROFILE_WITH_SPACES),
        strings(&[
            NOTARYTOOL_SUBCOMMAND,
            NOTARYTOOL_SUBMIT_SUBCOMMAND,
            PINNED_INPUT_PATH,
            NOTARYTOOL_FORCE_ARGUMENT,
            NOTARYTOOL_KEYCHAIN_PROFILE_ARGUMENT,
            PROFILE_WITH_SPACES,
            NOTARYTOOL_WAIT_ARGUMENT,
            NOTARYTOOL_TIMEOUT_ARGUMENT,
            NOTARYTOOL_TIMEOUT_VALUE,
            NOTARYTOOL_OUTPUT_FORMAT_ARGUMENT,
            NOTARYTOOL_JSON_FORMAT,
            NOTARYTOOL_NO_PROGRESS_ARGUMENT,
        ])
    );
}

#[test]
fn ticket_and_gatekeeper_commands_keep_the_exact_target_path() {
    for operation in [STAPLER_STAPLE_SUBCOMMAND, STAPLER_VALIDATE_SUBCOMMAND] {
        assert_eq!(
            arguments::stapler_arguments(operation, PATH_WITH_SPACES),
            strings(&[
                STAPLER_SUBCOMMAND,
                operation,
                STAPLER_VERBOSE_ARGUMENT,
                PATH_WITH_SPACES,
            ])
        );
    }
    assert_eq!(
        arguments::assessment_arguments(PATH_WITH_SPACES),
        strings(&[
            SPCTL_ASSESS_ARGUMENT,
            SPCTL_TYPE_ARGUMENT,
            SPCTL_EXECUTE_TYPE,
            SPCTL_VERBOSE_ARGUMENT,
            PATH_WITH_SPACES,
        ])
    );
}
