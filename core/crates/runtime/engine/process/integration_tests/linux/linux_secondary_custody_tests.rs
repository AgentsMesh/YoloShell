use super::*;

const FIRST_DIGEST_BYTE: u8 = 0x31;
const SECOND_DIGEST_BYTE: u8 = 0x32;

#[test]
fn composite_identity_is_stable_and_binds_both_authorities() {
    let containment = Sha256Digest::repeated(FIRST_DIGEST_BYTE);
    let slave = LinuxPtySlaveIdentity {
        device: 1,
        inode: 2,
        rdev: 3,
    };
    let first = composite_digest(containment, slave).unwrap();
    assert_eq!(first, composite_digest(containment, slave).unwrap());
    assert_ne!(
        first,
        composite_digest(Sha256Digest::repeated(SECOND_DIGEST_BYTE), slave).unwrap()
    );
    assert_ne!(
        first,
        composite_digest(
            containment,
            LinuxPtySlaveIdentity {
                inode: slave.inode + 1,
                ..slave
            }
        )
        .unwrap()
    );
}
