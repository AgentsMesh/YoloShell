use super::{FileIdentity, FileTimestamp};

pub fn largest_for_test() -> FileIdentity {
    let nanoseconds = i64::try_from(crate::runtime_constants::NANOSECONDS_PER_SECOND - 1).unwrap();
    FileIdentity {
        device: u64::MAX,
        inode: u64::MAX,
        length: u64::MAX,
        modified: FileTimestamp::new(i64::MIN, nanoseconds),
        changed: FileTimestamp::new(i64::MAX, nanoseconds),
        birth: FileTimestamp::new(i64::MIN, nanoseconds),
    }
}
