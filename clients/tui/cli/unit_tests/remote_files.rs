use super::remote_fixture::{constants::*, *};
use std::{
    fs,
    os::unix::fs::{PermissionsExt, symlink},
    path::PathBuf,
};

#[test]
fn symlinks_or_hardlinks_cannot_supply_profiles_or_either_private_key() {
    for input in INPUTS {
        for symbolic in [false, true] {
            let fixture = Fixture::new();
            let path = fixture.path(input);
            let retained = fixture.root.path().join(RETAINED_FILE);
            let bytes = fs::read(&path).unwrap();
            if symbolic {
                fs::rename(&path, &retained).unwrap();
                symlink(&retained, &path).unwrap();
            } else {
                fs::hard_link(&path, &retained).unwrap();
            }
            fixture.rejected();
            assert_eq!(fs::read(&retained).unwrap(), bytes);
        }
    }
}

#[test]
fn loose_permissions_and_missing_files_are_rejected_without_repairing_credentials() {
    for input in INPUTS {
        let fixture = Fixture::new();
        let path = fixture.path(input);
        fs::set_permissions(&path, fs::Permissions::from_mode(PUBLIC_MODE)).unwrap();
        fixture.rejected();
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & MODE_MASK,
            PUBLIC_MODE
        );
        fs::rename(&path, fixture.root.path().join(RETAINED_FILE)).unwrap();
        fixture.rejected();
        assert!(!path.exists());
    }
}

#[test]
fn oversized_profiles_and_keys_are_rejected_before_parsing() {
    for input in INPUTS {
        let fixture = Fixture::new();
        let limit = match input {
            Input::Profile => yolo_hub_ssh_entry::MAX_PROFILE_BYTES,
            Input::AccountKey | Input::ClientKey => {
                crate::constants::MAX_PRIVATE_KEY_BYTES as usize
            }
        };
        fs::write(
            fixture.path(input),
            vec![OVERSIZED_BYTE; limit + EXCESS_BYTE],
        )
        .unwrap();
        let error = fixture.rejected();
        assert_eq!(
            error.to_string(),
            crate::constants::PRIVATE_REMOTE_FILE_REQUIRED
        );
    }
}

#[test]
fn every_remote_file_path_must_be_absolute() {
    let fixture = Fixture::new();
    for input in INPUTS {
        let profile = if matches!(input, Input::Profile) {
            PathBuf::from(RELATIVE_INPUT)
        } else {
            fixture.path(Input::Profile)
        };
        let account = if matches!(input, Input::AccountKey) {
            PathBuf::from(RELATIVE_INPUT)
        } else {
            fixture.path(Input::AccountKey)
        };
        let client = if matches!(input, Input::ClientKey) {
            PathBuf::from(RELATIVE_INPUT)
        } else {
            fixture.path(Input::ClientKey)
        };
        let error = match fixture.select_paths(profile, account, client) {
            Err(error) => error,
            Ok(_) => panic!(),
        };
        fixture.private_error(&error);
        let expected =
            yolo_paths::validation::validate_absolute(std::path::Path::new(RELATIVE_INPUT))
                .unwrap_err();
        assert_eq!(error.to_string(), expected.to_string());
    }
}
