use crate::staging::{FILES, PENDING, store_chunk};
use std::fs;
use yolo_hub_terminal_protocol::{PublicError, StageAttachmentRequest};
use yolo_paths::directory::PrivateDirectory;
use std::os::unix::fs::{PermissionsExt, symlink};
use uuid::Uuid;
use yolo_hub_identity::{HubId, RuntimeEpoch, RuntimeTarget, TerminalAddress, TerminalId};

const NAME: &str = "a space-猫.png";
const HEAD: &[u8] = b"\x89PNG\x00";
const TAIL: &[u8] = b"\r\n\xff";

fn chunk() -> StageAttachmentRequest {
    StageAttachmentRequest {
        target: RuntimeTarget {
            address: TerminalAddress {
                hub_id: HubId::new(),
                terminal_id: TerminalId::new(),
            },
            runtime_epoch: RuntimeEpoch::new(),
        },
        upload_id: Uuid::new_v4(),
        name: NAME.into(),
        offset: 0,
        data: HEAD.into(),
        eof: false,
    }
}

#[test]
fn chunks_are_resumable_and_only_publish_a_complete_private_file() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("upload");
    let dir = PrivateDirectory::ensure(&path).unwrap();
    let mut request = chunk();
    assert_eq!(store_chunk(&dir, &request).unwrap(), None);
    assert_eq!(store_chunk(&dir, &request).unwrap(), None);
    assert!(!path.join(FILES).exists());
    request.offset = HEAD.len() as u64;
    request.data = TAIL.into();
    request.eof = true;
    let reopened = PrivateDirectory::open(&path).unwrap();
    let published = store_chunk(&reopened, &request).unwrap().unwrap();
    assert_eq!(
        store_chunk(&dir, &request).unwrap(),
        Some(published.clone())
    );
    assert_eq!(fs::read(&published).unwrap(), [HEAD, TAIL].concat());
    assert_eq!(
        fs::metadata(published).unwrap().permissions().mode() & 0o777,
        0o600
    );
    request.data[0] ^= 1;
    assert_eq!(store_chunk(&dir, &request), Err(PublicError::Conflict));
}

#[test]
fn incomplete_writes_can_be_retried_and_conflicting_names_offsets_and_data_are_rejected() {
    let temp = tempfile::tempdir().unwrap();
    let dir = PrivateDirectory::ensure(&temp.path().join("upload")).unwrap();
    let mut request = chunk();
    store_chunk(&dir, &request).unwrap();
    request.offset = HEAD.len() as u64 + 1;
    assert_eq!(store_chunk(&dir, &request), Err(PublicError::InvalidCursor));
    request.offset = 0;
    request.name = "other.png".into();
    assert_eq!(store_chunk(&dir, &request), Err(PublicError::Conflict));
    request.name = NAME.into();
    request.data = [HEAD, TAIL].concat();
    request.eof = true;
    let file = store_chunk(&dir, &request).unwrap().unwrap();
    assert_eq!(fs::read(&file).unwrap(), request.data);
    request.data.pop();
    assert_eq!(store_chunk(&dir, &request), Err(PublicError::Conflict));
}

#[test]
fn a_replaced_upload_file_does_not_modify_its_referent() {
    let temp = tempfile::tempdir().unwrap();
    let dir = PrivateDirectory::ensure(&temp.path().join("upload")).unwrap();
    let pending = dir.ensure_descendant(&dir.path().join(PENDING)).unwrap();
    let outside = temp.path().join("other");
    fs::write(&outside, TAIL).unwrap();
    symlink(&outside, pending.path().join(NAME)).unwrap();
    let request = chunk();
    assert!(store_chunk(&dir, &request).is_err());
    assert_eq!(fs::read(outside).unwrap(), TAIL);
}

#[test]
fn filenames_are_single_components_and_empty_files_are_supported() {
    let mut request = chunk();
    for bad in [
        "",
        ".",
        "..",
        "../../foo",
        "dir/name",
        "name/",
        "a\nb",
        "a\0b",
    ] {
        request.name = bad.into();
        assert_eq!(
            request.validate(),
            Err(PublicError::InvalidMetadata),
            "{bad:?}"
        );
    }
    request.name = NAME.into();
    request.data.clear();
    assert_eq!(request.validate(), Err(PublicError::InvalidPayload));
    request.eof = true;
    request.validate().unwrap();
    let temp = tempfile::tempdir().unwrap();
    let dir = PrivateDirectory::ensure(&temp.path().join("upload")).unwrap();
    let path = store_chunk(&dir, &request).unwrap().unwrap();
    assert_eq!(fs::metadata(path).unwrap().len(), 0);
}
