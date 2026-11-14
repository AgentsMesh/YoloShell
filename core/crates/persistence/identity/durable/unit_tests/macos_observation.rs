use super::*;

const FILE_ID: u64 = 257;
const DEVICE: libc::dev_t = 41;
const NEXT_ID: u64 = FILE_ID + 1;
const BIRTH_SECONDS: i64 = 1_700_000_000;

fn stat() -> libc::stat {
    // SAFETY: stat contains only integers and byte arrays.
    let mut value: libc::stat = unsafe { std::mem::zeroed() };
    value.st_mode = libc::S_IFDIR;
    value.st_dev = DEVICE;
    value.st_ino = FILE_ID;
    value.st_birthtime = BIRTH_SECONDS;
    value
}

#[test]
fn mac_observation_rejects_negative_birth_and_conflicting_attribute_identity() {
    let before = parse_observation(&stat()).unwrap();
    verify_attributes(before, FILE_ID, before.birth).unwrap();
    assert!(matches!(
        verify_attributes(before, NEXT_ID, before.birth),
        Err(Error::Changed)
    ));
    assert!(matches!(
        verify_attributes(
            before,
            FILE_ID,
            Birth {
                seconds: BIRTH_SECONDS + 1,
                ..before.birth
            }
        ),
        Err(Error::Changed)
    ));
    let mut negative = stat();
    negative.st_birthtime_nsec = -1;
    assert!(matches!(
        parse_observation(&negative),
        Err(Error::Invalid(_))
    ));
    let mut unsupported = stat();
    unsupported.st_mode = libc::S_IFSOCK;
    assert!(matches!(
        parse_observation(&unsupported),
        Err(Error::Unsupported(_))
    ));
}

#[test]
fn mac_backend_accepts_only_exact_apfs_filesystem_name() {
    // SAFETY: statfs contains only integers and byte arrays.
    let mut value: libc::statfs = unsafe { std::mem::zeroed() };
    assert!(matches!(
        validate_filesystem(&value),
        Err(Error::Unsupported(_))
    ));
    for (target, byte) in value.f_fstypename.iter_mut().zip(APFS_NAME) {
        *target = *byte as libc::c_char;
    }
    validate_filesystem(&value).unwrap();
    value.f_fstypename[APFS_NAME.len() - 1] = b'x' as libc::c_char;
    assert!(matches!(
        validate_filesystem(&value),
        Err(Error::Unsupported(_))
    ));
}
