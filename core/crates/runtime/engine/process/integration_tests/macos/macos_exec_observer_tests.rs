use super::*;

const DIFFERENT_PROCESS_OFFSET: u32 = 1;
const DIFFERENT_GENERATION_OFFSET: u64 = 1;

#[test]
fn exact_exec_event_is_accepted() {
    let target = current_identity();
    let event = event(target.pid(), u16::default(), libc::NOTE_EXEC);
    validate_event(&event, &target).unwrap();
}

#[test]
fn exit_event_is_rejected_even_when_exec_is_also_present() {
    let target = current_identity();
    let event = event(
        target.pid(),
        u16::default(),
        libc::NOTE_EXEC | libc::NOTE_EXIT,
    );
    assert!(validate_event(&event, &target).is_err());
}

#[test]
fn malformed_event_fields_are_rejected() {
    let target = current_identity();
    let cases = [
        event(
            target.pid() + DIFFERENT_PROCESS_OFFSET,
            u16::default(),
            libc::NOTE_EXEC,
        ),
        libc::kevent {
            filter: libc::EVFILT_READ,
            ..event(target.pid(), u16::default(), libc::NOTE_EXEC)
        },
        event(target.pid(), libc::EV_ERROR, libc::NOTE_EXEC),
        event(target.pid(), u16::default(), u32::default()),
    ];
    for candidate in cases {
        assert!(validate_event(&candidate, &target).is_err());
    }
}

#[test]
fn changed_process_identity_is_rejected() {
    let target = current_identity();
    let changed = target.with_generation(
        target
            .unique_id()
            .checked_add(DIFFERENT_GENERATION_OFFSET)
            .unwrap(),
    );
    let event = event(changed.pid(), u16::default(), libc::NOTE_EXEC);
    assert!(validate_event(&event, &changed).is_err());
    assert!(MacosExecObserver::register(changed).is_err());
}

#[test]
fn observer_deadline_expires_without_exec() {
    let observer = MacosExecObserver::register(current_identity()).unwrap();
    assert!(observer.wait_until(Instant::now()).is_err());
}

fn current_identity() -> LiveProcessIdentity {
    let identity = super::super::ProcessIdentity::capture(std::process::id()).unwrap();
    LiveProcessIdentity::capture(identity).unwrap().unwrap()
}

fn event(pid: u32, flags: u16, notes: u32) -> libc::kevent {
    libc::kevent {
        ident: pid as usize,
        filter: libc::EVFILT_PROC,
        flags,
        fflags: notes,
        data: libc::intptr_t::default(),
        udata: std::ptr::null_mut(),
    }
}
