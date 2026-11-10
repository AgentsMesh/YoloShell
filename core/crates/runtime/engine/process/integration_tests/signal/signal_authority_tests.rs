use super::*;

#[cfg(target_os = "macos")]
#[test]
fn destructive_access_captures_platform_capability_availability() {
    let unavailable = crate::process::macos_signal::with_test_capability(
        false,
        DestructiveSessionAccess::capture,
    );
    assert!(unavailable.require().is_err());

    let available =
        crate::process::macos_signal::with_test_capability(true, DestructiveSessionAccess::capture);
    assert!(available.require().is_ok());
}

#[cfg(target_os = "macos")]
#[test]
fn pty_spawn_preflight_requires_platform_capability() {
    let unavailable = crate::process::macos_signal::with_test_capability(
        false,
        require_pty_spawn_signal_capability,
    );
    assert!(unavailable.is_err());
    let available = crate::process::macos_signal::with_test_capability(
        true,
        require_pty_spawn_signal_capability,
    );
    assert!(available.is_ok());
}

#[cfg(not(target_os = "macos"))]
#[test]
fn destructive_access_fails_closed_without_platform_authority() {
    assert!(DestructiveSessionAccess::capture().require().is_err());
}

#[cfg(not(target_os = "macos"))]
#[test]
fn pty_spawn_preflight_does_not_require_macos_platform_capability() {
    assert!(require_pty_spawn_signal_capability().is_ok());
}
