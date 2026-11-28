use super::*;

const UUID_BYTE: u8 = 0x71;
const FILE_ID: u64 = 257;
const NEGATIVE_NANOSECONDS: i64 = -1;

fn volume() -> Volume {
    // SAFETY: attribute records contain only integer and byte fields.
    let mut value: Volume = unsafe { std::mem::zeroed() };
    value.length = size_of::<Volume>() as u32;
    value.returned.commonattr = libc::ATTR_CMN_RETURNED_ATTRS;
    value.returned.volattr = libc::ATTR_VOL_UUID;
    value.uuid = [UUID_BYTE; UUID_BYTES];
    value
}

fn object() -> Object {
    // SAFETY: attribute records contain only integer and byte fields.
    let mut value: Object = unsafe { std::mem::zeroed() };
    value.length = size_of::<Object>() as u32;
    value.returned.commonattr = OBJECT_FIELDS | libc::ATTR_CMN_RETURNED_ATTRS;
    value.file_id = FILE_ID;
    value
}

#[test]
fn missing_kernel_volume_attributes_cannot_claim_a_uuid() {
    assert_eq!(parse_volume(volume()).unwrap(), [UUID_BYTE; UUID_BYTES]);
    let mut short = volume();
    short.length -= 1;
    assert!(matches!(parse_volume(short), Err(Error::Unsupported(_))));
    let mut missing_common = volume();
    missing_common.returned.commonattr = u32::default();
    assert!(matches!(
        parse_volume(missing_common),
        Err(Error::Unsupported(_))
    ));
    let mut missing_uuid = volume();
    missing_uuid.returned.volattr = u32::default();
    assert!(matches!(
        parse_volume(missing_uuid),
        Err(Error::Unsupported(_))
    ));
}

#[test]
fn missing_kernel_object_fields_and_negative_birth_fraction_are_rejected() {
    let (id, birth) = parse_object(object()).unwrap();
    assert_eq!(id, FILE_ID);
    assert_eq!(birth.nanoseconds, u32::default());
    let mut short = object();
    short.length -= 1;
    assert!(matches!(parse_object(short), Err(Error::Unsupported(_))));
    for missing in [
        libc::ATTR_CMN_FILEID,
        libc::ATTR_CMN_CRTIME,
        libc::ATTR_CMN_RETURNED_ATTRS,
    ] {
        let mut incomplete = object();
        incomplete.returned.commonattr &= !missing;
        assert!(matches!(
            parse_object(incomplete),
            Err(Error::Unsupported(_))
        ));
    }
    let mut negative = object();
    negative.birth.tv_nsec = NEGATIVE_NANOSECONDS;
    assert!(matches!(parse_object(negative), Err(Error::Invalid(_))));
}

#[test]
fn only_kernel_capability_absence_is_an_unsupported_backend() {
    for errno in [libc::ENOTSUP, libc::ENOSYS] {
        assert!(matches!(
            query_error(std::io::Error::from_raw_os_error(errno)),
            Error::Unsupported(_)
        ));
    }
    for errno in [libc::EACCES, libc::EBADF, libc::EIO, libc::EINTR] {
        match query_error(std::io::Error::from_raw_os_error(errno)) {
            Error::Io { source, .. } => assert_eq!(source.raw_os_error(), Some(errno)),
            other => panic!("I/O failure cannot become missing backend support: {other:?}"),
        }
    }
}
