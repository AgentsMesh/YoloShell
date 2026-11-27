pub const GIT_EXECUTABLE: &str = "/usr/bin/git";
pub const TAR_EXECUTABLE: &str = "/usr/bin/tar";
pub const GIT_NO_REPLACE_ARGUMENT: &str = "--no-replace-objects";
pub const GIT_QUIET_ARGUMENT: &str = "--quiet";
pub const GIT_CONFIG_ARGUMENT: &str = "-c";
pub const GIT_NO_FSMONITOR_CONFIG: &str = "core.fsmonitor=false";
pub const GIT_NO_UNTRACKED_CACHE_CONFIG: &str = "core.untrackedCache=false";
pub const GIT_NO_REVERSE_INDEX_CONFIG: &str = "pack.writeReverseIndex=false";
pub const GIT_HEAD_ARGUMENTS: &[&str] = &["rev-parse", "--verify", "HEAD^{commit}"];
pub const GIT_ROOT_ARGUMENTS: &[&str] = &["rev-parse", "--show-toplevel"];
pub const GIT_OBJECT_FORMAT_ARGUMENTS: &[&str] = &["rev-parse", "--show-object-format"];
pub const GIT_STATUS_ARGUMENTS: &[&str] = &["status", "--porcelain=v1", "--untracked-files=all"];
pub const GIT_INDEX_FLAGS_ARGUMENTS: &[&str] = &["ls-files", "-v", "-z"];
pub const GIT_INDEX_RECORD_SEPARATOR: u8 = 0;
pub const GIT_INDEX_TAG_SEPARATOR: u8 = b' ';
pub const GIT_SKIP_WORKTREE_TAG: u8 = b'S';
pub const GIT_INIT_VERB: &str = concat!("in", "it");
pub const GIT_INIT_ARGUMENTS: &[&str] =
    &[GIT_INIT_VERB, "--bare", GIT_QUIET_ARGUMENT, "--template="];
pub const GIT_OBJECT_FORMAT_PREFIX: &str = "--object-format=";
pub const GIT_DIRECTORY_PREFIX: &str = "--git-dir=";
pub const GIT_METADATA_DIRECTORY_NAME: &str = ".git";
pub const GIT_ARCHIVE_ARGUMENT: &str = "archive";
pub const GIT_ARCHIVE_FORMAT_ARGUMENT: &str = "--format=tar";
pub const GIT_ARCHIVE_OUTPUT_PREFIX: &str = "--output=";
pub const GIT_PACK_OBJECTS_ARGUMENTS: &[&str] = &["pack-objects", "--revs"];
pub const GIT_INDEX_PACK_ARGUMENTS: &[&str] = &["index-pack", "--strict"];
pub const GIT_FSCK_ARGUMENTS: &[&str] = &["fsck", "--strict", "--no-dangling"];
pub const GIT_REV_LIST_ARGUMENTS: &[&str] = &[
    "rev-list",
    "--objects",
    "--missing=error",
    GIT_QUIET_ARGUMENT,
];
pub const GIT_PACK_DIRECTORY: &str = "objects/pack";
pub const GIT_PACK_PREFIX: &str = "pack";
pub const GIT_PACK_FILE_PREFIX: &str = "pack-";
pub const GIT_PACK_EXTENSION: &str = ".pack";
pub const GIT_REVISION_LINE_ENDING: &str = "\n";
pub const TAR_EXTRACT_ARGUMENTS: &[&str] = &["-xf"];
pub const TAR_DIRECTORY_ARGUMENT: &str = "-C";
pub const SUPPORTED_OBJECT_FORMATS: &[&str] = &["sha1", "sha256"];
pub const GIT_SHA1_FORMAT: &str = "sha1";
pub const SNAPSHOT_COMMAND_TIMEOUT: Duration = Duration::from_secs(120);
pub const WORKSPACE_DIRECTORY_ERROR: &str = "release workspace must be a directory";
pub const WORKSPACE_ROOT_ERROR: &str = "release workspace must be the Git root";
pub const WORKSPACE_DIRTY_ERROR: &str = "release workspace is not clean";
pub const WORKSPACE_INDEX_FLAGS_ERROR: &str = "release workspace index uses hidden change flags";
pub const WORKSPACE_INDEX_OUTPUT_ERROR: &str = "malformed Git index flag output";
pub const WORKSPACE_CHANGED_ERROR: &str = "release workspace changed while candidate gates ran";
use std::time::Duration;
