use yolo_release_contracts::{
    ARCHIVE_NAME, OUTPUT_FILE_NAMES, PRODUCT_NAME, PROVENANCE_NAME, RELEASE_EVIDENCE_CLI_NAME,
};
const VALID_IMAGE_DIGEST: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const IMAGE_BOUNDARY_STEP: u64 = 1;

#[test]
fn release_file_inventory_is_exact_and_product_named() {
    assert_eq!(PRODUCT_NAME, "Yolo");
    assert_eq!(ARCHIVE_NAME, "Yolo.zip");
    assert_eq!(PROVENANCE_NAME, "Yolo.provenance.json");
    assert_eq!(OUTPUT_FILE_NAMES.len(), 3);
    assert!(OUTPUT_FILE_NAMES.contains(&ARCHIVE_NAME));
    assert!(OUTPUT_FILE_NAMES.contains(&PROVENANCE_NAME));
}

#[test]
fn release_cli_identity_is_stable() {
    assert_eq!(RELEASE_EVIDENCE_CLI_NAME, "yolo-release-evidence");
}

#[test]
fn linux_runtime_publication_uses_the_live_image_byte_bound_without_widening_other_tools() {
    assert_eq!(
        yolo_release_contracts::LINUX_RUNTIME_MAX_BYTES,
        yolo_build_image::MAX_EXECUTABLE_IMAGE_BYTES
    );
    let maximum = yolo_release_contracts::LINUX_RUNTIME_MAX_BYTES;
    let image = yolo_build_image::BuildImage::new(VALID_IMAGE_DIGEST.into(), maximum).unwrap();
    assert_eq!(image.size_bytes(), maximum);
    assert_eq!(
        yolo_build_image::BuildImage::new(VALID_IMAGE_DIGEST.into(), maximum + IMAGE_BOUNDARY_STEP),
        Err(yolo_build_image::BuildImageError::InvalidSize)
    );
    assert!(
        yolo_release_contracts::MAX_DOCKER_CLIENT_BYTES
            < yolo_release_contracts::LINUX_RUNTIME_MAX_BYTES
    );
    assert!(
        yolo_release_contracts::MAX_BAZEL_LAUNCHER_BYTES
            < yolo_release_contracts::LINUX_RUNTIME_MAX_BYTES
    );
}
