use super::{constants::*, files};
use crate::{evidence, fingerprint::Fingerprint};
use std::{
    fs::{File, FileTimes},
    io::Write,
    os::unix::fs::PermissionsExt,
    time::{Duration, SystemTime},
};
use yolo_build_image::MAX_EXECUTABLE_IMAGE_BYTES;

#[test]
fn retained_file_hash_is_verified_before_and_after_reading() {
    let root = tempfile::tempdir().unwrap();
    let mut file = files::image(&root.path().join(DESCRIPTOR_FILE), SOURCE);
    let mut observations = usize::default();
    let (_, image) = evidence::capture(&mut file, |_| {
        observations += STEP as usize;
        Ok(())
    })
    .unwrap();
    assert_eq!(image.sha256(), DIGEST);
    assert_eq!(image.size_bytes(), SIZE);
    assert_eq!(observations, POST_HASH_VERIFICATION);
}

#[test]
fn in_place_mutation_or_named_replacement_during_capture_never_returns_old_hash() {
    for replace in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join(DESCRIPTOR_FILE);
        let other = root.path().join(REPLACEMENT_FILE);
        let mut file = files::image(&path, SOURCE);
        let _replacement = files::image(&other, REPLACEMENT);
        let mut count = usize::default();
        let result = evidence::capture(&mut file, |retained| {
            count += STEP as usize;
            if count == POST_HASH_VERIFICATION {
                if replace {
                    std::fs::rename(&other, &path)?;
                } else {
                    let mut changed = File::options().write(true).open(&path)?;
                    changed.write_all(REPLACEMENT)?;
                    changed.set_times(FileTimes::new().set_modified(
                        SystemTime::UNIX_EPOCH + Duration::from_secs(CHANGED_TIMESTAMP_SECONDS),
                    ))?;
                }
            }
            anyhow::ensure!(
                Fingerprint::inspect(&std::fs::symlink_metadata(&path)?)?
                    == Fingerprint::inspect(&retained.metadata()?)?,
                crate::constants::IMAGE_CHANGED
            );
            Ok(())
        });
        assert!(result.is_err());
    }
}

#[test]
fn non_executable_empty_directory_and_oversized_candidates_fail_before_hashing() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join(DESCRIPTOR_FILE);
    let mut file = files::image(&path, SOURCE);
    file.set_permissions(std::fs::Permissions::from_mode(NON_EXECUTABLE_MODE))
        .unwrap();
    assert!(evidence::capture(&mut file, |_| panic!()).is_err());
    file.set_permissions(std::fs::Permissions::from_mode(PRIVATE_EXECUTABLE_MODE))
        .unwrap();
    for size in [u64::default(), MAX_EXECUTABLE_IMAGE_BYTES + STEP] {
        file.set_len(size).unwrap();
        assert!(evidence::capture(&mut file, |_| panic!()).is_err());
    }
    let mut directory = File::open(root.path()).unwrap();
    assert!(evidence::capture(&mut directory, |_| panic!()).is_err());
}

#[test]
fn failed_mapped_image_validation_never_publishes_a_digest() {
    let root = tempfile::tempdir().unwrap();
    for failed_at in [STEP as usize, POST_HASH_VERIFICATION] {
        let path = root.path().join(format!("{DESCRIPTOR_FILE}{failed_at}"));
        let mut file = files::image(&path, SOURCE);
        let mut observed = usize::default();
        let result = evidence::capture(&mut file, |_| {
            observed += STEP as usize;
            anyhow::ensure!(observed != failed_at, crate::constants::IMAGE_CHANGED);
            Ok(())
        });
        assert!(result.is_err());
    }
}
