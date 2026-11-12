#[path = "../test_support/constants.rs"]
mod constants;
use constants::*;
use yolo_build_image::*;

#[test]
fn canonical_public_image_round_trips_without_process_or_path_fields() {
    let image = BuildImage::new(DIGEST.into(), SIZE).unwrap();
    assert_eq!(image.sha256(), DIGEST);
    assert_eq!(image.size_bytes(), SIZE);
    assert_eq!(image.fields().schema_version, BUILD_IMAGE_SCHEMA_VERSION);
    assert_eq!(
        BuildImage::try_from(BuildImageFields::from(image.clone())),
        Ok(image.clone())
    );
    let encoded = serde_json::to_vec(&image).unwrap();
    assert_eq!(
        serde_json::from_slice::<BuildImage>(&encoded).unwrap(),
        image
    );
    assert_eq!(
        serde_json::to_value(image)
            .unwrap()
            .as_object()
            .unwrap()
            .len(),
        FIELD_COUNT
    );
}

#[test]
fn schema_digest_and_size_boundaries_are_validated_on_construction_and_decode() {
    for digest in [
        String::new(),
        DIGEST.to_uppercase(),
        DIGEST.replacen('b', "g", STEP as usize),
        DIGEST[..DIGEST.len() - STEP as usize].into(),
        format!("{DIGEST}a"),
    ] {
        assert_eq!(
            BuildImage::new(digest, SIZE),
            Err(BuildImageError::InvalidDigest)
        );
    }
    for size in [u64::default(), MAX_EXECUTABLE_IMAGE_BYTES + STEP] {
        assert_eq!(
            BuildImage::new(DIGEST.into(), size),
            Err(BuildImageError::InvalidSize)
        );
    }
    assert!(BuildImage::new(DIGEST.into(), MAX_EXECUTABLE_IMAGE_BYTES).is_ok());
    let fields = BuildImageFields {
        schema_version: UNKNOWN_SCHEMA,
        sha256: DIGEST.into(),
        size_bytes: SIZE,
    };
    assert_eq!(
        BuildImage::try_from(fields.clone()),
        Err(BuildImageError::UnsupportedSchema)
    );
    assert!(serde_json::from_slice::<BuildImage>(&serde_json::to_vec(&fields).unwrap()).is_err());
}

#[test]
fn private_identity_claims_and_duplicate_json_fields_are_not_accepted() {
    for invalid in [DUPLICATE, UNKNOWN_FIELD] {
        assert!(serde_json::from_str::<BuildImage>(invalid).is_err());
    }
    for error in [
        BuildImageError::UnsupportedSchema,
        BuildImageError::InvalidDigest,
        BuildImageError::InvalidSize,
    ] {
        assert!(!error.to_string().is_empty());
        assert!(std::error::Error::source(&error).is_none());
    }
}
