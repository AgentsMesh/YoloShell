use std::time::Duration;

pub const DISTRIBUTE_OPERATION: &str = "distribute";
pub const NOTARY_PROFILE_ARGUMENT: &str = "notary-profile";
pub const ACCEPTANCE_RECEIPT_ARGUMENT: &str = "acceptance-receipt";
pub const DEVELOPER_IDENTITY: &str =
    "Developer ID Application: Beijing SginalRender Technology Co., Ltd (DCK4SSVH2H)";
pub const DEVELOPER_TEAM_ID: &str = "DCK4SSVH2H";
pub const DEVELOPER_REQUIREMENT_TEAM_PREFIX: &str =
    "anchor apple generic and certificate leaf[subject.OU] = \"";
pub const DEVELOPER_REQUIREMENT_COMMON_NAME_PREFIX: &str =
    "\" and certificate leaf[subject.CN] = \"";
pub const APP_ARCHIVE_PATH: &str = "Yolo.app";
pub const SIGNED_ARCHIVE_PATHS: &[&str] = &[
    "Yolo.app/Contents/MacOS/Yolo",
    "Yolo.app/Contents/Helpers/yolo",
];
pub const EXECUTABLE_ARCHIVE_DIRECTORIES: &[&str] =
    &["Yolo.app/Contents/MacOS/", "Yolo.app/Contents/Helpers/"];
pub const CODE_RESOURCES_ARCHIVE_PATH: &str = "Yolo.app/Contents/_CodeSignature/CodeResources";
pub const NOTARY_TICKET_ARCHIVE_PATH: &str = "Yolo.app/Contents/CodeResources";
pub const NOTARY_SUBMISSION_NAME: &str = ".yolo-notary-submission.zip";
pub const PINNED_INPUT_DESCRIPTOR: libc::c_int = 3;
pub const PINNED_INPUT_PATH: &str = "/dev/fd/3";
pub const XCRUN_EXECUTABLE: &str = "/usr/bin/xcrun";
pub const SPCTL_EXECUTABLE: &str = "/usr/sbin/spctl";
pub const CODESIGN_FORCE_ARGUMENT: &str = "--force";
pub const CODESIGN_SIGN_ARGUMENT: &str = "--sign";
pub const CODESIGN_OPTIONS_ARGUMENT: &str = "--options";
pub const CODESIGN_RUNTIME_OPTION: &str = "runtime";
pub const CODESIGN_TIMESTAMP_ARGUMENT: &str = "--timestamp";
pub const CODESIGN_VERBOSE_ARGUMENT: &str = "--verbose=2";
pub const CODESIGN_REQUIREMENT_PREFIX: &str = "-R=";
pub const CODESIGN_REMOVE_SIGNATURE_ARGUMENT: &str = "--remove-signature";
pub const NOTARYTOOL_SUBCOMMAND: &str = "notarytool";
pub const NOTARYTOOL_SUBMIT_SUBCOMMAND: &str = "submit";
pub const NOTARYTOOL_FORCE_ARGUMENT: &str = "--force";
pub const NOTARYTOOL_KEYCHAIN_PROFILE_ARGUMENT: &str = "--keychain-profile";
pub const NOTARYTOOL_WAIT_ARGUMENT: &str = "--wait";
pub const NOTARYTOOL_TIMEOUT_ARGUMENT: &str = "--timeout";
pub const NOTARYTOOL_TIMEOUT_VALUE: &str = "30m";
pub const NOTARYTOOL_OUTPUT_FORMAT_ARGUMENT: &str = "--output-format";
pub const NOTARYTOOL_JSON_FORMAT: &str = "json";
pub const NOTARYTOOL_NO_PROGRESS_ARGUMENT: &str = "--no-progress";
pub const NOTARY_ACCEPTED_STATUS: &str = "Accepted";
pub const STAPLER_SUBCOMMAND: &str = "stapler";
pub const STAPLER_STAPLE_SUBCOMMAND: &str = "staple";
pub const STAPLER_VALIDATE_SUBCOMMAND: &str = "validate";
pub const STAPLER_VERBOSE_ARGUMENT: &str = "-v";
pub const SPCTL_ASSESS_ARGUMENT: &str = "--assess";
pub const SPCTL_TYPE_ARGUMENT: &str = "--type";
pub const SPCTL_EXECUTE_TYPE: &str = "execute";
pub const SPCTL_VERBOSE_ARGUMENT: &str = "--verbose=4";
pub const DITTO_CREATE_ARGUMENT: &str = "-c";
pub const DITTO_ZIP_ARGUMENT: &str = "-k";
pub const DITTO_KEEP_PARENT_ARGUMENT: &str = "--keepParent";
pub const MAX_NOTARY_PROFILE_BYTES: usize = 128;
pub const MAX_NOTARY_TICKET_BYTES: u64 = 1024 * 1024;
pub const DEVELOPER_RELEASE_COMMAND_TIMEOUT: Duration = Duration::from_secs(3_600);
pub const DEVELOPER_RELEASE_PROCESS_ERROR: &str = "Developer release subprocess failed";
pub const DEVELOPER_RELEASE_COMMAND_PREFIX: &str = "yolo-developer-release-command-";
pub const DEVELOPER_RELEASE_BASELINE_PREFIX: &str = "yolo-developer-release-baseline-";
pub const NORMALIZED_EXECUTABLE_PREFIX: &str = "normalized-executable-";
pub const DEVELOPER_RELEASE_INVENTORY_ERROR: &str =
    "Developer release changed a non-signing bundle entry";
pub const NOTARY_PROFILE_ERROR: &str = "notary profile is invalid";
pub const NOTARY_RESPONSE_ERROR: &str = "notary service did not accept the submission";
pub const DEVELOPER_RELEASE_EXECUTABLE_CHANGED_ERROR: &str =
    "Developer release changed executable content outside its code signature";
pub const PUBLIC_DISTRIBUTION_ACCEPTANCE_ERROR: &str =
    "public distribution requires authenticated acceptance evidence";
