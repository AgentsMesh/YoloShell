pub(crate) const HOOKS: &str = "hooks";
pub(crate) const COMMAND: &str = "command";
pub(crate) const TYPE: &str = "type";
pub(crate) const TIMEOUT: &str = "timeout";
pub(crate) const MATCHER: &str = "matcher";
#[cfg(test)]
pub(crate) const CODEX_FILE: &str = "hooks.json";
pub(crate) const BACKUP_PREFIX: &str = "yolo-hooks-backup-";
// Never use a provider-scanned extension here: providers such as Grok merge
// every *.json file in their hook directory, so a backup must not look like a
// live hook profile.
pub(crate) const BACKUP_SUFFIX: &str = ".json.bak";
pub(crate) const LEGACY_BACKUP_SUFFIX: &str = ".json";
pub(crate) const ABSENT_REVISION: &str = "absent-provider-directory";
pub(crate) const STAGING_PREFIX: &str = ".yolo-hooks-staging-";
pub(crate) const OWNER_DIRECTORY_MODE: u32 = 0o700;
pub(crate) const OWNER_FILE_MODE: u32 = 0o600;
pub(crate) const PERMISSION_MASK: u32 = 0o7777;
pub(crate) const UNSAFE_WRITE_MODE: u32 = 0o022;
pub(crate) const SPECIAL_MODE: u32 = 0o7000;
pub(crate) const STICKY_MODE: u32 = 0o1000;
pub(crate) const ROOT_UID: u32 = 0;
pub(crate) const SINGLE_LINK: u64 = 1;
pub(crate) const UNSAFE_CONFIG: &str = "provider configuration ownership or permissions are unsafe";
pub(crate) const MAX_CONFIG_BYTES: u64 = 1024 * 1024;
pub(crate) const INVALID_CONFIG: &str = "provider hooks configuration is malformed";
pub(crate) const CONFIG_CHANGED: &str = "provider hooks configuration changed after preview";
pub(crate) const CONFIG_DISABLED: &str = "provider configuration restricts or disables hooks";
pub(crate) const INVALID_HOME: &str = "provider configuration directory is unavailable";
