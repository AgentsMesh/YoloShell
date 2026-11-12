use super::*;
use std::path::Path;

#[test]
fn boot_and_process_identity_are_available() {
    let current_boot = boot_id();
    assert!(!current_boot.is_empty());
    assert_eq!(
        boot_identity_relation("known", "known"),
        BootIdentityRelation::Same
    );
    assert_eq!(
        boot_identity_relation("known", "other"),
        BootIdentityRelation::Different
    );
    assert_eq!(
        boot_identity_relation(&format!("{UNKNOWN_BOOT_ID_PREFIX}1"), "known"),
        BootIdentityRelation::Unknown
    );
    assert_eq!(
        boot_identity_relation("", "known"),
        BootIdentityRelation::Unknown
    );
    assert_eq!(
        boot_identity_relation("known", ""),
        BootIdentityRelation::Unknown
    );
    assert_eq!(
        boot_identity_relation("known", &format!("{UNKNOWN_BOOT_ID_PREFIX}2")),
        BootIdentityRelation::Unknown
    );
    let pid = std::process::id();
    let start_time = process_start_time(pid).unwrap();
    assert!(start_time > 0);
    assert!(process_identity_matches(pid, start_time));
    assert!(!process_identity_matches(pid, start_time.wrapping_add(1)));
    assert!(is_process_alive(std::process::id()));
    assert!(!is_process_alive(0));
    assert!(!is_process_alive(u32::MAX));
}

#[test]
fn parses_signals_without_or_with_prefix() {
    #[cfg(unix)]
    {
        assert_eq!(parse_signal("INT").unwrap(), libc::SIGINT);
        assert_eq!(parse_signal("sigterm").unwrap(), libc::SIGTERM);
        assert!(parse_signal("made-up").is_err());
    }
}

#[test]
fn detached_spawn_signaling_and_tree_termination_are_observable() {
    #[cfg(unix)]
    {
        let mut child = spawn_detached(
            Path::new("/bin/sleep"),
            &["30".into()],
            DetachedStandardError::Null,
        )
        .unwrap();
        let pid = child.id();
        let root = ProcessIdentity::capture(pid).unwrap();
        let mut tree = CapturedProcessTree::capture(root).unwrap();
        assert!(is_process_alive(pid));
        assert!(root.signal(libc::SIGCONT).unwrap());
        tree.terminate(libc::SIGTERM).unwrap();
        let status = child.wait().unwrap();
        assert!(!status.success());
        assert!(!is_process_alive(u32::MAX));
        for name in ["KILL", "HUP", "QUIT", "TSTP", "CONT", "WINCH"] {
            assert!(parse_signal(name).is_ok());
        }
    }
}

#[test]
fn invalid_and_missing_process_targets_fail_without_signaling_process_groups() {
    #[cfg(unix)]
    {
        assert!(process_start_time(0).is_err());
        assert!(process_start_time(u32::MAX).is_err());
        assert!(
            spawn_detached(
                Path::new("/definitely/missing/yolo-command"),
                &[],
                DetachedStandardError::Null,
            )
            .is_err()
        );
        assert!(ProcessIdentity::capture(0).is_err());
        assert!(ProcessIdentity::capture(u32::MAX).is_err());
    }
}
