use crate::LocalProfile;
use std::path::Path;

#[cfg(target_os = "macos")]
mod aliases;
mod linux;

const BASE: &str = "/private/profile-test/Application Support";
const STATE: &str = "/private/profile-test/Application Support/Yolo/hub-local";
const HOST: &str = "/private/profile-test/Application Support/Yolo/Runtime";
const INVALID_PATH: &str = "/private/bad\npath";

#[test]
fn application_support_composition_is_pure_and_uses_the_native_directory_contract() {
    let profile = LocalProfile::from_application_support(Path::new(BASE)).unwrap();
    assert_eq!(profile.state, Path::new(STATE));
    assert_eq!(profile.host_home, Path::new(HOST));
    assert_eq!(
        serde_json::from_slice::<LocalProfile>(&serde_json::to_vec(&profile).unwrap()).unwrap(),
        profile
    );
}

#[test]
fn invalid_profile_never_becomes_a_different_default() {
    let good = LocalProfile::from_application_support(Path::new(BASE)).unwrap();
    for path in ["relative", "", INVALID_PATH] {
        let mut invalid = good.clone();
        invalid.state = path.into();
        assert!(invalid.validate().is_err());
        assert!(LocalProfile::from_application_support(Path::new(path)).is_err());
    }
    let mut invalid = good.clone();
    invalid.host_home = invalid.state.clone();
    assert!(invalid.validate().is_err());
    invalid.host_home = invalid.state.join("nested");
    assert!(invalid.validate().is_err());
    let mut invalid = good;
    invalid.schema_version += crate::constants::SCHEMA_VERSION;
    assert!(invalid.validate().is_err());
}

#[test]
fn unknown_fields_and_malformed_profiles_are_rejected() {
    let profile = LocalProfile::from_application_support(Path::new(BASE)).unwrap();
    let mut value = serde_json::to_value(profile).unwrap();
    value["helper"] = serde_json::Value::Bool(true);
    assert!(serde_json::from_value::<LocalProfile>(value).is_err());
    assert!(serde_json::from_slice::<LocalProfile>(b"{}").is_err());
}

#[cfg(target_os = "macos")]
#[test]
fn foundation_representation_is_checked_before_use() {
    for value in [c"", c"relative", c"/bad\npath", c"/\xFF"] {
        assert!(crate::macos::decode(value).is_err());
    }
    assert_eq!(
        crate::macos::decode(c"/private/目录").unwrap(),
        Path::new("/private/目录")
    );
}
