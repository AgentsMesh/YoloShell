#[cfg(all(target_os = "macos", not(test)))]
pub const COORDINATION_ROOT_PREFIX: &str = "/private/tmp/yolo-coordination-";
#[cfg(all(unix, not(target_os = "macos"), not(test)))]
pub const COORDINATION_ROOT_PREFIX: &str = "/tmp/yolo-coordination-";
pub const DIGEST_SEPARATOR: u8 = 0;
pub const LOCK_FILE_EXTENSION: &str = ".lock";
pub const CATALOG_ALIAS_DIRECTORY: &str = "catalog-aliases";
pub const CATALOG_ALIAS_DIGEST_DOMAIN: &[u8] = b"yolo-catalog-alias-v1";
pub const CATALOG_ALIAS_DIGEST_HEX_BYTES: usize = LOCK_DIGEST_HEX_BYTES;
pub const LIFECYCLE_LOCK_FILE: &str = ".lifecycle.lock";
pub const MAINTENANCE_OWNER_LOCK_FILE: &str = ".maintenance-owner.lock";
pub const SWEEP_LOCK_MAX_BYTES: u64 = std::mem::size_of::<i64>() as u64;
pub const SWEEP_DELETE_BUDGET: usize = 4;
pub const SWEEP_ENTRY_BUDGET: usize = 16;
pub const LOCK_DIGEST_BYTES: usize = 32;
pub(crate) const HEX_DIGITS_PER_BYTE: usize = 2;
pub const LOCK_DIGEST_HEX_BYTES: usize = LOCK_DIGEST_BYTES * HEX_DIGITS_PER_BYTE;
pub const MAX_COORDINATION_LOCK_BYTES: u64 = 128;
pub const HOST_HOME_SCOPE: &str = "host-home";
pub const HOST_ENDPOINT_SCOPE: &str = "host-endpoint";
pub const WORKER_SCOPE: &str = "worker";
pub const BROKER_INSTANCE_SCOPE: &str = "broker-instance";
pub const BROKER_LAUNCH_PUBLICATION_SCOPE: &str = "broker-launch-publication";
pub const BROKER_RECONCILE_SCOPE: &str = "broker-reconcile";
pub const SOCKET_MAINTENANCE_SCOPE: &str = "socket-maintenance";
pub const MANAGED_OVERLAP_ERROR: &str =
    "coordination storage must not overlap the managed namespace";
pub const LOCK_CLEANUP_FAILED: &str = "failed to reclaim a coordination lock";
pub const LOCK_PARENT_MISSING_ERROR: &str = "coordination lock has no parent";
pub const LOCK_PARENT_CHANGED_ERROR: &str = "coordination lock parent changed";
pub const SWEEP_OPEN_CONTEXT: &str = "open stale coordination lock";
#[cfg(target_os = "macos")]
pub const MACH_SERVICE_PREFIX: &str = "com.agentsmesh.yolo.coordination.";
#[cfg(target_os = "macos")]
pub const MACH_SERVICE_MAX_BYTES: usize = 128;
#[cfg(target_os = "macos")]
pub const MACH_PORT_RIGHT_RECEIVE: libc::c_int = 1;
#[cfg(target_os = "macos")]
pub const MACH_PORT_RELEASE_REFERENCE: libc::c_int = -1;
#[cfg(target_os = "macos")]
pub const BOOTSTRAP_SUCCESS: libc::kern_return_t = 0;
#[cfg(target_os = "macos")]
pub const BOOTSTRAP_NOT_PRIVILEGED: libc::kern_return_t = 1100;
#[cfg(target_os = "macos")]
pub const BOOTSTRAP_NAME_IN_USE: libc::kern_return_t = 1101;
#[cfg(target_os = "macos")]
pub const BOOTSTRAP_UNKNOWN_SERVICE: libc::kern_return_t = 1102;
#[cfg(target_os = "macos")]
pub const BOOTSTRAP_SERVICE_ACTIVE: libc::kern_return_t = 1103;
#[cfg(target_os = "macos")]
pub const CASE_INSENSITIVE_SUFFIX_ERROR: &str =
    "a missing path component on a case-insensitive volume must be ASCII";
#[cfg(target_os = "macos")]
pub const CASE_INSENSITIVE_VOLUME: libc::c_long = 0;
#[cfg(target_os = "macos")]
pub const CASE_SENSITIVE_VOLUME: libc::c_long = 1;

#[cfg(target_os = "macos")]
pub const MACOS_FIRMLINK_ALIASES: [(&str, &str); 19] = [
    (
        "/System/Library/PreinstalledAssetsV2",
        "/System/Volumes/Data/System/Library/PreinstalledAssetsV2",
    ),
    (
        "/System/Library/PreinstalledAssets",
        "/System/Volumes/Data/System/Library/PreinstalledAssets",
    ),
    (
        "/System/Library/AssetsV2",
        "/System/Volumes/Data/System/Library/AssetsV2",
    ),
    (
        "/System/Library/Assets",
        "/System/Volumes/Data/System/Library/Assets",
    ),
    (
        "/System/Library/CoreServices/CoreTypes.bundle/Contents/Library",
        "/System/Volumes/Data/System/Library/CoreServices/CoreTypes.bundle/Contents/Library",
    ),
    (
        "/System/Library/Caches",
        "/System/Volumes/Data/System/Library/Caches",
    ),
    (
        "/System/Library/Speech",
        "/System/Volumes/Data/System/Library/Speech",
    ),
    ("/AppleInternal", "/System/Volumes/Data/AppleInternal"),
    ("/Applications", "/System/Volumes/Data/Applications"),
    ("/Library", "/System/Volumes/Data/Library"),
    ("/Users", "/System/Volumes/Data/Users"),
    ("/Volumes", "/System/Volumes/Data/Volumes"),
    ("/cores", "/System/Volumes/Data/cores"),
    ("/opt", "/System/Volumes/Data/opt"),
    ("/pkg", "/System/Volumes/Data/pkg"),
    ("/private", "/System/Volumes/Data/private"),
    ("/usr/libexec/cups", "/System/Volumes/Data/usr/libexec/cups"),
    ("/usr/share/snmp", "/System/Volumes/Data/usr/share/snmp"),
    ("/usr/local", "/System/Volumes/Data/usr/local"),
];
