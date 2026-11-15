use super::*;

#[test]
fn missing_or_invalid_birth_identity_fails_closed() {
    assert!(!valid_birth(0, 0));
    assert!(!valid_birth(-1, 0));
    assert!(!valid_birth(
        1,
        i64::try_from(crate::runtime_constants::NANOSECONDS_PER_SECOND).unwrap()
    ));
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
#[test]
fn descriptor_capture_includes_a_birth_generation() {
    let file = tempfile::tempfile().unwrap();
    assert!(inspect(&file).is_ok());
}

#[cfg(target_os = "linux")]
#[test]
fn linux_device_encoding_preserves_major_and_minor_components() {
    const DEVICE_MAJOR: u32 = 259;
    const DEVICE_MINOR: u32 = 65_537;
    const EXPECTED_DEVICE: u64 = 268_501_761;
    assert_eq!(linux_device(DEVICE_MAJOR, DEVICE_MINOR), EXPECTED_DEVICE);
}
