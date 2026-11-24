use super::{bundle_fixture::*, memory_fs::FailurePoint};
use crate::remote::*;
use std::path::Path;
use yolo_distribution_publication_protocol::{
    CURRENT_FILE_NAME, DIRECTORY_MODE, MANIFEST_MODE, MAX_PUBLICATION_BYTES,
};

#[tokio::test]
async fn selection_absence_is_distinct_from_permission_error_and_present_private_file() {
    let fixture = fixture().await;
    assert_eq!(
        fixture
            .connection
            .read_bundle_selection(Path::new(ROOT))
            .await
            .unwrap()
            .value,
        None
    );
    fixture.filesystem.insert_directory(ROOT, DIRECTORY_MODE);
    assert_eq!(
        fixture
            .connection
            .read_bundle_selection(Path::new(ROOT))
            .await
            .unwrap()
            .value,
        None
    );
    fixture.filesystem.fail_once(FailurePoint::Metadata);
    assert!(
        fixture
            .connection
            .read_bundle_selection(Path::new(ROOT))
            .await
            .is_err()
    );
    let path = Path::new(ROOT).join(CURRENT_FILE_NAME);
    fixture
        .filesystem
        .insert_file(path.to_str().unwrap(), SELECTION_BYTES, MANIFEST_MODE);
    let value = fixture
        .connection
        .read_bundle_selection(Path::new(ROOT))
        .await
        .unwrap();
    assert_eq!(value.identity.purpose, ChannelPurpose::FileRead);
    assert_eq!(value.value, Some(SELECTION_BYTES.to_vec()));
}

#[tokio::test]
async fn selection_rejects_links_unsafe_modes_and_oversize_payload_without_writes() {
    for kind in [Fault::Link, Fault::Mode, Fault::Size] {
        let fixture = fixture().await;
        fixture.filesystem.insert_directory(ROOT, DIRECTORY_MODE);
        let path = Path::new(ROOT).join(CURRENT_FILE_NAME);
        match kind {
            Fault::Link => fixture
                .filesystem
                .insert_link(path.to_str().unwrap(), super::constants::MEMORY_FILE),
            Fault::Mode => {
                fixture
                    .filesystem
                    .insert_file(path.to_str().unwrap(), SELECTION_BYTES, PARENT_MODE)
            }
            Fault::Size => fixture.filesystem.insert_file(
                path.to_str().unwrap(),
                &vec![u8::default(); MAX_PUBLICATION_BYTES + EXTRA_BYTE],
                MANIFEST_MODE,
            ),
        }
        assert!(
            fixture
                .connection
                .read_bundle_selection(Path::new(ROOT))
                .await
                .is_err()
        );
        assert_eq!(fixture.filesystem.writes(), usize::default());
    }
}
enum Fault {
    Link,
    Mode,
    Size,
}
