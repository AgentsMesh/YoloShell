use super::BootIdentityRelation;

pub(super) const MACOS_BOOT_PREFIX: &str = "macos:";
const SYSTEM_CALL_SUCCESS: libc::c_int = 0;
const NUL_BYTE: u8 = 0;
const NUL_TERMINATOR_BYTES: usize = 1;
const BOOT_SESSION_UUID_BYTES: usize = uuid::fmt::Hyphenated::LENGTH + NUL_TERMINATOR_BYTES;
pub(super) fn resolve() -> Option<String> {
    read_boot_session_uuid().map(format_session)
}

pub(super) fn relation(left: &str, right: &str) -> BootIdentityRelation {
    if !left.starts_with(MACOS_BOOT_PREFIX) || !right.starts_with(MACOS_BOOT_PREFIX) {
        return BootIdentityRelation::Different;
    }
    let (Some(left), Some(right)) = (parse(left), parse(right)) else {
        return BootIdentityRelation::Unknown;
    };
    if left == right {
        return BootIdentityRelation::Same;
    }
    BootIdentityRelation::Different
}

fn parse(value: &str) -> Option<uuid::Uuid> {
    let value = value.strip_prefix(MACOS_BOOT_PREFIX)?;
    if value.len() != uuid::fmt::Hyphenated::LENGTH {
        return None;
    }
    uuid::Uuid::parse_str(value)
        .ok()
        .filter(|session| !session.is_nil())
}

fn read_boot_session_uuid() -> Option<uuid::Uuid> {
    let key = std::ffi::CString::new(super::super::constants::MACOS_BOOT_SESSION_UUID_NAME).ok()?;
    let mut bytes = [u8::default(); BOOT_SESSION_UUID_BYTES];
    let mut length = bytes.len();
    // SAFETY: bytes owns the writable storage described by length.
    let result = unsafe {
        libc::sysctlbyname(
            key.as_ptr(),
            bytes.as_mut_ptr().cast(),
            &mut length,
            std::ptr::null_mut(),
            usize::default(),
        )
    };
    if result != SYSTEM_CALL_SUCCESS || length > bytes.len() {
        return None;
    }
    parse_session_bytes(&bytes[..length])
}

fn parse_session_bytes(bytes: &[u8]) -> Option<uuid::Uuid> {
    let value = bytes.strip_suffix(&[NUL_BYTE])?;
    if value.len() != uuid::fmt::Hyphenated::LENGTH {
        return None;
    }
    uuid::Uuid::parse_str(std::str::from_utf8(value).ok()?)
        .ok()
        .filter(|session| !session.is_nil())
}

fn format_session(session: uuid::Uuid) -> String {
    format!("{MACOS_BOOT_PREFIX}{session}")
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../integration_tests/boot_identity_macos_tests.rs"]
mod tests;
