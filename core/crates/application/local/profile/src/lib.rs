//! Local service directories, independent of terminal UI and process startup.
mod constants;
#[cfg(target_os = "macos")]
mod foundation;
#[cfg(any(target_os = "linux", test))]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
mod profile;
pub use profile::LocalProfile;

#[cfg(target_os = "macos")]
pub fn current() -> anyhow::Result<LocalProfile> {
    LocalProfile::from_application_support(&macos::application_support_directory()?)
}

#[cfg(target_os = "linux")]
pub fn current() -> anyhow::Result<LocalProfile> {
    linux::current()
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
pub fn current() -> anyhow::Result<LocalProfile> {
    anyhow::bail!(constants::AUTOMATIC_UNSUPPORTED)
}

#[cfg(all(test, not(feature = "yolo-integration-tests")))]
#[path = "../unit_tests/lib.rs"]
mod unit_tests;
