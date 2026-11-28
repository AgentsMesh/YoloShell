#[path = "constants.rs"]
mod constants;
use crate::LocalProfile;
use constants::*;
use std::path::{Path, PathBuf};

fn profile(state: PathBuf, host_home: PathBuf) -> LocalProfile {
    LocalProfile {
        schema_version: crate::constants::SCHEMA_VERSION,
        state,
        host_home,
    }
}

#[test]
fn standard_aliases_cannot_select_the_same_directory_for_both_services() {
    for &(visible, physical) in STANDARD_ALIAS_ROOTS {
        for (state, host_home) in [(visible, physical), (physical, visible)] {
            assert!(profile(state.into(), host_home.into()).validate().is_err());
        }
    }
}

#[test]
fn standard_aliases_cannot_hide_parent_child_overlap_in_either_direction() {
    for &(visible, physical) in STANDARD_ALIAS_ROOTS {
        for (parent, child) in [(visible, physical), (physical, visible)] {
            let parent = PathBuf::from(parent);
            let child = Path::new(child).join(NESTED_DIRECTORY);
            assert!(profile(parent.clone(), child.clone()).validate().is_err());
            assert!(profile(child, parent).validate().is_err());
        }
    }
}

#[test]
fn alias_siblings_remain_valid_and_preserve_original_unicode_paths() {
    for &(visible, physical) in STANDARD_ALIAS_ROOTS {
        for (state, host_home) in [(visible, physical), (physical, visible)] {
            let profile = profile(
                Path::new(state)
                    .join(UNICODE_DIRECTORY)
                    .join(STATE_DIRECTORY),
                Path::new(host_home)
                    .join(UNICODE_DIRECTORY)
                    .join(HOST_DIRECTORY),
            );
            let before = profile.clone();
            profile.validate().unwrap();
            assert_eq!(profile, before);
        }
    }
}
