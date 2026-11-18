use super::*;

const DEVICE: u64 = 41;
const INODE: u64 = 257;
const BIRTH: i64 = 1_700_000_000;
const STEP: u64 = 1;

#[test]
fn capture_fence_rejects_every_changed_live_object_attribute() {
    let before = Observation {
        device: DEVICE,
        inode: INODE,
        kind: ObjectKind::Directory,
        birth: Birth {
            seconds: BIRTH,
            nanoseconds: u32::default(),
        },
    };
    before.verify(before).unwrap();
    let changed = [
        Observation {
            device: DEVICE + STEP,
            ..before
        },
        Observation {
            inode: INODE + STEP,
            ..before
        },
        Observation {
            kind: ObjectKind::RegularFile,
            ..before
        },
        Observation {
            birth: Birth {
                seconds: BIRTH + STEP as i64,
                ..before.birth
            },
            ..before
        },
        Observation {
            birth: Birth {
                nanoseconds: STEP as u32,
                ..before.birth
            },
            ..before
        },
    ];
    for after in changed {
        assert!(matches!(before.verify(after), Err(Error::Changed)));
    }
}
