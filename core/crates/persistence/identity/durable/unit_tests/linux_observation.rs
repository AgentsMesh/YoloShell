use super::*;

const INODE: u64 = 257;
const SECONDS: i64 = -1;
const DEVICE_MAJOR: u32 = 8;
const DEVICE_MINOR: u32 = 1;

fn response() -> libc::statx {
    // SAFETY: statx contains only integer fields and arrays, all valid when zero.
    let mut output: libc::statx = unsafe { std::mem::zeroed() };
    output.stx_mask = STATX_FIELDS;
    output.stx_mode = libc::S_IFDIR as u16;
    output.stx_ino = INODE;
    output.stx_dev_major = DEVICE_MAJOR;
    output.stx_dev_minor = DEVICE_MINOR;
    output.stx_btime.tv_sec = SECONDS;
    output
}

#[test]
fn kernel_observation_requires_all_fields_and_normalized_nanoseconds() {
    let valid = response();
    let observed = parse_observation(&valid).unwrap();
    assert_eq!(observed.device, libc::makedev(DEVICE_MAJOR, DEVICE_MINOR));
    assert_eq!(observed.inode, INODE);
    assert_eq!(observed.birth.seconds, SECONDS);
    for missing in [libc::STATX_TYPE, libc::STATX_INO, libc::STATX_BTIME] {
        let mut incomplete = response();
        incomplete.stx_mask &= !missing;
        assert!(matches!(
            parse_observation(&incomplete),
            Err(Error::Unsupported(_))
        ));
    }
    let mut invalid = response();
    invalid.stx_btime.tv_nsec = NANOSECONDS_PER_SECOND;
    assert!(matches!(
        parse_observation(&invalid),
        Err(Error::Invalid(_))
    ));
    invalid.stx_btime.tv_nsec = NANOSECONDS_PER_SECOND - 1;
    assert!(parse_observation(&invalid).is_ok());
    invalid.stx_mode = libc::S_IFSOCK as u16;
    assert!(matches!(
        parse_observation(&invalid),
        Err(Error::Unsupported(_))
    ));
}
