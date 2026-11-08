use super::kind;
use crate::{Error, ObjectKind};

const PRIVATE_DIRECTORY: u32 = 0o700;
const PRIVATE_FILE: u32 = 0o600;

#[test]
fn permissions_do_not_change_object_kind_and_special_files_never_gain_authority() {
    assert_eq!(
        kind(u32::from(libc::S_IFDIR) | PRIVATE_DIRECTORY).unwrap(),
        ObjectKind::Directory
    );
    assert_eq!(
        kind(u32::from(libc::S_IFREG) | PRIVATE_FILE).unwrap(),
        ObjectKind::RegularFile
    );
    for special in [
        libc::S_IFLNK,
        libc::S_IFIFO,
        libc::S_IFSOCK,
        libc::S_IFCHR,
        libc::S_IFBLK,
    ] {
        assert!(matches!(
            kind(u32::from(special)),
            Err(Error::Unsupported(_))
        ));
    }
}
