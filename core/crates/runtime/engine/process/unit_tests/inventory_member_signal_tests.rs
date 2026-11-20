use super::*;

const VALID_PID: u32 = 41;
const VALID_BIRTH: u64 = 99;
const INVALID_NEGATIVE_SIGNAL: i32 = -1;

fn target() -> ProcessIdentity {
    ProcessIdentity {
        pid: VALID_PID,
        start_time: VALID_BIRTH,
    }
}

#[test]
fn signal_validation_rejects_probe_invalid_and_force_signals_before_inspection() {
    for signal in [
        i32::default(),
        INVALID_NEGATIVE_SIGNAL,
        i32::MAX,
        libc::SIGKILL,
    ] {
        let group = ProcessGroupIdentity::verified(VALID_PID, Vec::new());
        assert!(signal_process_group_member(group, target(), signal).is_err());
    }
}

#[test]
fn unbound_process_locators_cannot_be_used_as_member_authority() {
    for expected in [
        ProcessIdentity {
            pid: u32::default(),
            ..target()
        },
        ProcessIdentity {
            pid: u32::MAX,
            ..target()
        },
        ProcessIdentity {
            start_time: u64::default(),
            ..target()
        },
    ] {
        let group = ProcessGroupIdentity::verified(VALID_PID, Vec::new());
        assert!(signal_process_group_member(group, expected, libc::SIGINT).is_err());
    }
}

#[test]
fn supported_foreground_signals_have_the_same_non_force_vocabulary() {
    for signal in [
        libc::SIGINT,
        libc::SIGTERM,
        libc::SIGHUP,
        libc::SIGQUIT,
        libc::SIGTSTP,
        libc::SIGCONT,
        libc::SIGWINCH,
    ] {
        validate(target(), signal).unwrap();
    }
}
