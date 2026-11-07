use super::{bundle_fixture::*, constants::*};
use crate::remote::*;
use yolo_distribution_bundle::{BIN_DIRECTORY, RELEASES_DIRECTORY};
use yolo_distribution_publication_protocol::{DIRECTORY_MODE, EXECUTABLE_MODE};

#[tokio::test]
async fn typed_verification_uses_one_cached_authenticated_uid_for_staged_and_published_roles() {
    let fixture = fixture().await;
    let request = request();
    fixture.connection.stage_bundle(&request).await.unwrap();
    for role in ArtifactRole::ALL {
        let result = fixture
            .connection
            .verify_bundle_executable(&request, BundleExecutableLocation::Staged, role)
            .await
            .unwrap();
        assert_eq!(result.identity.purpose, ChannelPurpose::FileRead);
        assert_eq!(
            result.identity.connection_id,
            fixture.connection.info().connection_id
        );
    }
    let release = request
        .descriptor()
        .release_directory(request.install_root());
    for directory in [
        request.install_root().join(RELEASES_DIRECTORY),
        release.clone(),
        release.join(BIN_DIRECTORY),
    ] {
        fixture
            .filesystem
            .insert_directory(directory.to_str().unwrap(), DIRECTORY_MODE);
    }
    for role in ArtifactRole::ALL {
        let path = request
            .descriptor()
            .executable_path(request.install_root(), role);
        fixture.filesystem.insert_file(
            path.to_str().unwrap(),
            request.payload(role),
            EXECUTABLE_MODE,
        );
        fixture
            .connection
            .verify_bundle_executable(&request, BundleExecutableLocation::Published, role)
            .await
            .unwrap();
    }
    assert_eq!(fixture.filesystem.uid_probes(), EXPECTED_PROBES);
}

#[derive(Clone, Copy)]
enum Fault {
    RootMode,
    BinMode,
    FileMode,
    FileOwner,
    Link,
}

#[tokio::test]
async fn identical_bytes_do_not_authorize_an_unsafe_executable_or_private_directory() {
    for fault in [
        Fault::RootMode,
        Fault::BinMode,
        Fault::FileMode,
        Fault::FileOwner,
        Fault::Link,
    ] {
        let fixture = fixture().await;
        let request = request();
        fixture.connection.stage_bundle(&request).await.unwrap();
        let path = request.executable(ArtifactRole::Runtime);
        match fault {
            Fault::RootMode => fixture.filesystem.insert_directory(ROOT, PARENT_MODE),
            Fault::BinMode => fixture.filesystem.insert_directory(
                request
                    .stage_directory()
                    .join(BIN_DIRECTORY)
                    .to_str()
                    .unwrap(),
                PARENT_MODE,
            ),
            Fault::FileMode => fixture.filesystem.insert_file(
                path.to_str().unwrap(),
                request.payload(ArtifactRole::Runtime),
                PARENT_MODE,
            ),
            Fault::FileOwner => fixture.filesystem.set_file_uid(
                path.to_str().unwrap(),
                MEMORY_UID + COUNTER_INCREMENT as u32,
            ),
            Fault::Link => fixture
                .filesystem
                .insert_link(path.to_str().unwrap(), MEMORY_FILE),
        }
        assert!(
            fixture
                .connection
                .verify_bundle_executable(
                    &request,
                    BundleExecutableLocation::Staged,
                    ArtifactRole::Runtime
                )
                .await
                .is_err()
        );
    }
}

#[tokio::test]
async fn uid_probe_rejects_malformed_or_failed_output_and_does_not_cache_failure() {
    for (stdout, stderr, status) in [
        (b"".as_slice(), b"".as_slice(), Some(0)),
        (b"501", b"", Some(0)),
        (b"0501\n", b"", Some(0)),
        (b"501\n502\n", b"", Some(0)),
        (b"4294967296\n", b"", Some(0)),
        (b"501\n", b"warning", Some(0)),
        (b"501\n", b"", Some(1)),
        (b"501\n", b"", None),
    ] {
        let fixture = fixture().await;
        let request = request();
        fixture.filesystem.set_uid_response(stdout, stderr, status);
        assert!(fixture.connection.stage_bundle(&request).await.is_err());
        assert!(
            fixture
                .connection
                .cleanup
                .retained_bundle_stages()
                .is_empty()
        );
        assert_eq!(fixture.filesystem.writes(), usize::default());
        fixture
            .filesystem
            .set_uid_response(format!("{MEMORY_UID}\n").as_bytes(), b"", Some(0));
        fixture.connection.stage_bundle(&request).await.unwrap();
        assert_eq!(fixture.filesystem.uid_probes(), RETRIED_PROBES);
    }
}

#[tokio::test]
async fn private_root_uid_must_match_the_authenticated_session_uid() {
    let fixture = fixture().await;
    let request = request();
    fixture.filesystem.set_uid_response(
        format!("{}\n", MEMORY_UID + COUNTER_INCREMENT as u32).as_bytes(),
        b"",
        Some(0),
    );
    assert!(fixture.connection.stage_bundle(&request).await.is_err());
    assert!(
        fixture
            .connection
            .cleanup
            .retained_bundle_stages()
            .is_empty()
    );
    assert_eq!(fixture.filesystem.writes(), usize::default());
}

const EXPECTED_PROBES: usize = 1;
const RETRIED_PROBES: usize = 2;
