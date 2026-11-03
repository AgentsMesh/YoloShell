use super::*;
use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};

const EVIDENCE_NAME: &str = "source-evidence";
const EVIDENCE_BYTES: &[u8] = b"preserve until owned container absence is proven";
const PRIVATE_MODE: u32 = 0o600;
const GROUP_WRITABLE_MODE: u32 = 0o620;
const MODE_MASK: u32 = 0o777;
const CID_TARGET: &str = "actual-cid";
enum InvalidRecord {
    Symlink,
    HardLink,
    Directory,
    Fifo,
    Writable,
    Oversize,
    Encoding,
    Uppercase,
}

#[test]
fn untrusted_cid_record_never_authorizes_removal_or_snapshot_destruction() {
    for invalid in [
        InvalidRecord::Symlink,
        InvalidRecord::HardLink,
        InvalidRecord::Directory,
        InvalidRecord::Fifo,
        InvalidRecord::Writable,
        InvalidRecord::Oversize,
        InvalidRecord::Encoding,
        InvalidRecord::Uppercase,
    ] {
        let snapshot =
            GateSnapshot::fixture_with_source(&[(EVIDENCE_NAME, EVIDENCE_BYTES)]).unwrap();
        let root = snapshot.root_path();
        let evidence = snapshot.source().display_path().join(EVIDENCE_NAME);
        let mode = fs::metadata(&evidence).unwrap().permissions().mode() & MODE_MASK;
        let engine = TestEngine::default();
        let expected = contract(&snapshot);
        let cid = expected.command.cid_file.clone();
        let observed = inspection(&expected);
        let owner = Owner::prepare(&snapshot, &engine, expected, None).unwrap();
        *engine.current.borrow_mut() = Some(observed);
        let target = snapshot.temporary().display_path().join(CID_TARGET);
        match invalid {
            InvalidRecord::Symlink | InvalidRecord::HardLink => {
                fs::write(&target, CONTAINER_ID).unwrap();
                fs::set_permissions(&target, fs::Permissions::from_mode(PRIVATE_MODE)).unwrap();
                if matches!(invalid, InvalidRecord::Symlink) {
                    symlink(&target, &cid).unwrap();
                } else {
                    fs::hard_link(&target, &cid).unwrap();
                }
            }
            InvalidRecord::Directory => fs::create_dir(&cid).unwrap(),
            InvalidRecord::Fifo => yolo_release_gate_test_support::support::fifo(&cid),
            InvalidRecord::Writable => {
                fs::write(&cid, CONTAINER_ID).unwrap();
                fs::set_permissions(&cid, fs::Permissions::from_mode(GROUP_WRITABLE_MODE)).unwrap();
            }
            InvalidRecord::Oversize => fs::write(&cid, CONTAINER_ID.repeat(2)).unwrap(),
            InvalidRecord::Encoding => fs::write(&cid, b"\xff").unwrap(),
            InvalidRecord::Uppercase => fs::write(&cid, CONTAINER_ID.to_uppercase()).unwrap(),
        }
        assert!(owner.finish(Err(anyhow::anyhow!(CLIENT_FAILURE))).is_err());
        assert!(engine.removals.borrow().is_empty());
        drop(snapshot);
        assert_eq!(fs::read(&evidence).unwrap(), EVIDENCE_BYTES);
        assert_eq!(
            fs::metadata(evidence).unwrap().permissions().mode() & MODE_MASK,
            mode
        );
        if target.exists() {
            assert_eq!(fs::read(&target).unwrap(), CONTAINER_ID.as_bytes());
            assert_eq!(
                fs::metadata(target).unwrap().permissions().mode() & MODE_MASK,
                PRIVATE_MODE
            );
        }
        remove_preserved(root);
    }
}

#[test]
fn empty_reserved_cid_uses_exact_name_recovery_and_complete_cid_uses_exact_id() {
    let complete = format!("{CONTAINER_ID}\n");
    for (bytes, acknowledged) in [
        (b"".as_slice(), false),
        (b"\n", false),
        (complete.as_bytes(), true),
    ] {
        let snapshot = GateSnapshot::fixture().unwrap();
        let root = snapshot.root_path();
        let engine = TestEngine::default();
        let expected = contract(&snapshot);
        let name = expected.command.name.clone();
        let cid = expected.command.cid_file.clone();
        let observed = inspection(&expected);
        let owner = Owner::prepare(&snapshot, &engine, expected, None).unwrap();
        *engine.current.borrow_mut() = Some(observed);
        fs::write(cid, bytes).unwrap();
        assert!(owner.finish(Err(anyhow::anyhow!(CLIENT_FAILURE))).is_err());
        assert_eq!(*engine.removals.borrow(), [CONTAINER_ID]);
        let recovery = if acknowledged {
            Lookup::Id(CONTAINER_ID.into())
        } else {
            Lookup::Name(name.clone())
        };
        assert_eq!(
            *engine.lookups.borrow(),
            [
                Lookup::Name(name),
                recovery,
                Lookup::Id(CONTAINER_ID.into()),
            ]
        );
        drop(snapshot);
        assert!(!root.exists());
    }
}

#[test]
fn preexisting_cid_path_is_collision_and_cannot_be_cleaned_by_new_owner() {
    let snapshot = GateSnapshot::fixture().unwrap();
    let engine = TestEngine::default();
    let expected = contract(&snapshot);
    let cid = expected.command.cid_file.clone();
    fs::write(&cid, CONTAINER_ID).unwrap();
    let result = Owner::prepare(&snapshot, &engine, expected, None);
    assert!(result.is_err());
    assert_eq!(fs::read(cid).unwrap(), CONTAINER_ID.as_bytes());
    assert!(engine.removals.borrow().is_empty());
}
