use crate::{
    Error,
    model::{Birth, UUID_BYTES},
};
use std::mem::size_of;
use std::os::fd::{AsRawFd, BorrowedFd};

#[repr(C, packed(4))]
struct Volume {
    length: u32,
    returned: libc::attribute_set_t,
    uuid: [u8; UUID_BYTES],
}

#[repr(C, packed(4))]
struct Object {
    length: u32,
    returned: libc::attribute_set_t,
    birth: libc::timespec,
    file_id: u64,
}

const OBJECT_FIELDS: u32 = libc::ATTR_CMN_CRTIME | libc::ATTR_CMN_FILEID;

pub(super) fn volume_uuid(descriptor: BorrowedFd<'_>) -> Result<[u8; UUID_BYTES], Error> {
    let mut attributes = request();
    attributes.volattr = libc::ATTR_VOL_INFO | libc::ATTR_VOL_UUID;
    // SAFETY: Volume is entirely integers/byte arrays, whose zero values are valid.
    let mut output: Volume = unsafe { std::mem::zeroed() };
    query(
        descriptor,
        &mut attributes,
        (&mut output as *mut Volume).cast(),
        size_of::<Volume>(),
    )?;
    parse_volume(output)
}

fn parse_volume(output: Volume) -> Result<[u8; UUID_BYTES], Error> {
    if output.length as usize != size_of::<Volume>()
        || output.returned.commonattr & libc::ATTR_CMN_RETURNED_ATTRS == u32::default()
        || output.returned.volattr & libc::ATTR_VOL_UUID == u32::default()
    {
        return Err(Error::Unsupported("APFS volume UUID attribute"));
    }
    Ok(output.uuid)
}

pub(super) fn object(descriptor: BorrowedFd<'_>) -> Result<(u64, Birth), Error> {
    let mut attributes = request();
    attributes.commonattr |= OBJECT_FIELDS;
    // SAFETY: Object contains only integers, so zero is a valid initial value.
    let mut output: Object = unsafe { std::mem::zeroed() };
    query(
        descriptor,
        &mut attributes,
        (&mut output as *mut Object).cast(),
        size_of::<Object>(),
    )?;
    parse_object(output)
}

fn parse_object(output: Object) -> Result<(u64, Birth), Error> {
    let required = OBJECT_FIELDS | libc::ATTR_CMN_RETURNED_ATTRS;
    if output.length as usize != size_of::<Object>()
        || output.returned.commonattr & required != required
    {
        return Err(Error::Unsupported("APFS file ID and birth attributes"));
    }
    let nanoseconds =
        u32::try_from(output.birth.tv_nsec).map_err(|_| Error::Invalid("birth nanoseconds"))?;
    Ok((
        output.file_id,
        Birth {
            seconds: output.birth.tv_sec,
            nanoseconds,
        },
    ))
}

fn request() -> libc::attrlist {
    libc::attrlist {
        bitmapcount: libc::ATTR_BIT_MAP_COUNT,
        reserved: u16::default(),
        commonattr: libc::ATTR_CMN_RETURNED_ATTRS,
        volattr: u32::default(),
        dirattr: u32::default(),
        fileattr: u32::default(),
        forkattr: u32::default(),
    }
}

fn query(
    descriptor: BorrowedFd<'_>,
    attributes: &mut libc::attrlist,
    output: *mut libc::c_void,
    size: usize,
) -> Result<(), Error> {
    // SAFETY: callers supply their exact repr(C, packed(4)) attribute output size.
    let status = unsafe {
        libc::fgetattrlist(
            descriptor.as_raw_fd(),
            std::ptr::from_mut(attributes).cast(),
            output,
            size,
            libc::FSOPT_PACK_INVAL_ATTRS,
        )
    };
    if status == libc::c_int::default() {
        return Ok(());
    }
    Err(query_error(std::io::Error::last_os_error()))
}

fn query_error(source: std::io::Error) -> Error {
    if matches!(source.raw_os_error(), Some(libc::ENOTSUP | libc::ENOSYS)) {
        return Error::Unsupported("APFS persistent attributes");
    }
    Error::Io {
        operation: "read APFS persistent attributes",
        source,
    }
}

#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../../../integration_tests/macos_attributes.rs"]
mod integration_tests;

#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../../../unit_tests/macos_attributes.rs"]
mod unit_tests;
