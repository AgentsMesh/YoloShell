use crate::linux::from_environment;
use std::{ffi::OsStr, os::unix::ffi::OsStrExt, path::Path};

const HOME: &str = "/home/profile-test";
const STATE_HOME: &str = "/srv/profile-state";
const XDG_STATE: &str = "/srv/profile-state/yolo/hub-local";
const XDG_HOST: &str = "/srv/profile-state/yolo/runtime";
const HOME_STATE: &str = "/home/profile-test/.local/state/yolo/hub-local";
const HOME_HOST: &str = "/home/profile-test/.local/state/yolo/runtime";
const INVALID_PATHS: &[&str] = &["relative", "/bad\npath", "/state/../other"];
const NON_UTF8_PATH: &[u8] = b"/state/\xff";

#[test]
fn explicit_xdg_state_home_is_used_without_requiring_home() {
    let profile = from_environment(Some(OsStr::new(STATE_HOME)), None).unwrap();
    assert_eq!(profile.state, Path::new(XDG_STATE));
    assert_eq!(profile.host_home, Path::new(XDG_HOST));
    assert_eq!(
        from_environment(Some(OsStr::new(STATE_HOME)), Some(OsStr::new("relative"))).unwrap(),
        profile
    );
}

#[test]
fn unset_or_empty_xdg_state_home_uses_home_state_directory() {
    for state_home in [None, Some(OsStr::new(""))] {
        let profile = from_environment(state_home, Some(OsStr::new(HOME))).unwrap();
        assert_eq!(profile.state, Path::new(HOME_STATE));
        assert_eq!(profile.host_home, Path::new(HOME_HOST));
    }
}

#[test]
fn invalid_configured_directories_do_not_fall_back_to_another_profile() {
    for invalid in INVALID_PATHS {
        assert!(from_environment(Some(OsStr::new(invalid)), Some(OsStr::new(HOME))).is_err());
        assert!(from_environment(None, Some(OsStr::new(invalid))).is_err());
    }
    let invalid = OsStr::from_bytes(NON_UTF8_PATH);
    assert!(from_environment(Some(invalid), Some(OsStr::new(HOME))).is_err());
    assert!(from_environment(None, Some(invalid)).is_err());
}

#[test]
fn missing_or_empty_home_cannot_choose_a_relative_profile() {
    assert!(from_environment(None, None).is_err());
    assert!(from_environment(None, Some(OsStr::new(""))).is_err());
}
