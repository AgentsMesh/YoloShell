use crate::{ARCHIVE_NAME, CHECKSUM_NAME};
use std::time::Duration;

pub const DELIVER_OPERATION: &str = "deliver";
pub const CANDIDATE_DIRECTORY_ARGUMENT: &str = "candidate-directory";
pub const APP_NAME: &str = "Yolo.app";
pub const DELIVERY_OUTPUT_FILE_NAMES: &[&str] = &[APP_NAME, ARCHIVE_NAME, CHECKSUM_NAME];
pub const DELIVERY_TEMPORARY_PREFIX: &str = ".yolo-delivery-stage-";
pub const DELIVERY_RECEIPT_PREFIX: &str = ".yolo-delivery-receipt-";
pub const DELIVERY_RECEIPT_TEMPORARY_PREFIX: &str = ".yolo-delivery-receipt-stage-";
pub const EXTRACTION_DIRECTORY_NAME: &str = ".yolo-delivery-extraction";
pub const DELIVERY_RECEIPT_SCHEMA: u16 = 3;
pub const MAX_DELIVERY_RECEIPT_BYTES: u64 = 16 * 1024;
pub const DITTO_EXECUTABLE: &str = "/usr/bin/ditto";
pub const DITTO_EXTRACT_ARGUMENT: &str = "-x";
pub const DITTO_KEEP_PARENT_ARGUMENT: &str = "-k";
pub const DITTO_NO_RESOURCE_FORK_ARGUMENT: &str = "--norsrc";
pub const DITTO_NO_EXTENDED_ATTRIBUTES_ARGUMENT: &str = "--noextattr";
pub const DITTO_NO_ACL_ARGUMENT: &str = "--noacl";
pub const DITTO_NO_QUARANTINE_ARGUMENT: &str = "--noqtn";
pub const DITTO_ARGUMENTS: &[&str] = &[
    DITTO_EXTRACT_ARGUMENT,
    DITTO_KEEP_PARENT_ARGUMENT,
    DITTO_NO_RESOURCE_FORK_ARGUMENT,
    DITTO_NO_EXTENDED_ATTRIBUTES_ARGUMENT,
    DITTO_NO_ACL_ARGUMENT,
    DITTO_NO_QUARANTINE_ARGUMENT,
];
pub const CODESIGN_EXECUTABLE: &str = "/usr/bin/codesign";
pub const CODESIGN_VERIFY_ARGUMENT: &str = "--verify";
pub const CODESIGN_DEEP_ARGUMENT: &str = "--deep";
pub const CODESIGN_STRICT_ARGUMENT: &str = "--strict";
pub const CODESIGN_ARGUMENTS: &[&str] = &[
    CODESIGN_VERIFY_ARGUMENT,
    CODESIGN_DEEP_ARGUMENT,
    CODESIGN_STRICT_ARGUMENT,
];
pub const DELIVERY_COMMAND_TIMEOUT: Duration = Duration::from_secs(120);
pub const DELIVERY_SYSTEM_CALL_SUCCESS: libc::c_int = 0;
pub const DELIVERY_OPEN_FAILURE: libc::c_int = -1;
pub const DELIVERY_UNLINK_FILE_FLAGS: libc::c_int = 0;
pub const DELIVERY_DESTINATION_NAME_ERROR: &str = "delivery output name is invalid";
pub const DELIVERY_DESTINATION_UTF8_ERROR: &str = "delivery output name must be UTF-8";
pub const DELIVERY_DESTINATION_PARENT_ERROR: &str = "delivery output has no parent";
pub const DELIVERY_STAGE_ERROR: &str = "stage independent delivery directory";
pub const DELIVERY_EXTRACTION_ERROR: &str = "stage delivery extraction directory";
pub const DELIVERY_RENAME_ERROR: &str = "atomically publish independent delivery";
pub const DELIVERY_APP_RENAME_ERROR: &str = "install verified extracted application";
pub const DELIVERY_ROOT_ENTRIES_ERROR: &str = "delivery extraction root entries mismatch";
pub const DELIVERY_OUTPUT_ENTRIES_ERROR: &str = "delivered output entries mismatch";
pub const DELIVERY_TREE_MISMATCH_ERROR: &str = "extracted tree differs from ZIP inventory";
pub const DELIVERY_TREE_ENTRY_ERROR: &str = "inspect extracted tree entry";
pub const DELIVERY_FILE_CREATE_ERROR: &str = "create staged delivery file";
pub const DELIVERY_FILE_OPEN_ERROR: &str = "open staged delivery file";
pub const DELIVERY_FILE_MODE_ERROR: &str = "delivered file mode mismatch";
pub const DELIVERY_FILE_TYPE_ERROR: &str = "delivered output must be a regular file";
pub const DELIVERY_FILE_OWNER_ERROR: &str = "delivered file owner mismatch";
pub const DELIVERY_FILE_LINK_ERROR: &str = "delivered file must have one link";
pub const DELIVERY_FILE_CHANGED_ERROR: &str = "staged delivery file changed";
pub const DELIVERY_CHECKSUM_MISMATCH_ERROR: &str = "delivered checksum mismatch";
pub const DELIVERY_ARCHIVE_DIGEST_ERROR: &str = "delivered archive digest mismatch";
pub const DELIVERY_ARCHIVE_INVENTORY_ERROR: &str = "delivered archive inventory mismatch";
pub const DELIVERY_CANDIDATE_CHANGED_ERROR: &str = "verified delivery candidate changed";
pub const DELIVERY_EXTRACT_DESCRIPTION: &str = "extract verified release archive";
pub const DELIVERY_SIGNATURE_DESCRIPTION: &str = "verify delivered application signature";
pub const DELIVERY_PROCESS_FAILURE_ERROR: &str = "delivery subprocess failed";
pub const DELIVERY_TREE_TYPE_ERROR: &str = "extracted tree contains an unsupported entry";
pub const DELIVERY_TREE_OWNER_ERROR: &str = "extracted tree owner mismatch";
pub const DELIVERY_TREE_LINK_ERROR: &str = "extracted regular file must have one link";
pub const DELIVERY_CLEANUP_ERROR: &str = "clean private delivery staging directory";
pub const DELIVERY_SYNC_ERROR: &str = "sync private delivery tree";
pub const DELIVERY_EXISTING_OUTPUT_ERROR: &str =
    "existing delivery does not match the requested candidate";
pub const DELIVERY_COMMITTED_UNCONFIRMED_ERROR: &str =
    "delivery committed but final confirmation failed; retry the same delivery command";
pub const DELIVERY_PREPARED_UNCONFIRMED_ERROR: &str =
    "delivery prepared but publication was not confirmed; retry the same delivery command";
pub const DELIVERY_RECOVERY_CONFLICT_ERROR: &str =
    "delivery recovery state conflicts with the retained destination or stage";
pub const DELIVERY_RECEIPT_ERROR: &str = "validate durable delivery recovery receipt";
