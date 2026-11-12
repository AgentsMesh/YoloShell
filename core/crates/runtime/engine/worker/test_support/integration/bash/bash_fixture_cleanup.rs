use super::*;

#[path = "bash_fixture_cleanup_constants.rs"]
pub(super) mod constants;
#[path = "bash_locale_constants.rs"]
mod terminal;
use self::constants::*;

pub(super) fn fixture() -> Fixture {
    let mut command = CommandBuilder::new(terminal::BASH);
    command.env_clear();
    command.args(terminal::BASH_ARGS);
    for (key, value) in terminal::FIXTURE_ENV {
        command.env(key, value);
    }
    crate::worker::environment::configure(
        &mut command,
        &[(terminal::LC_ALL_ENV.into(), terminal::C_LOCALE.into())].into(),
    );
    let fixture = Fixture::with_command(command);
    wait_for_screen(&fixture, std::str::from_utf8(terminal::PROMPT).unwrap());
    lock_recover(&fixture.state.writer)
        .as_mut()
        .unwrap()
        .write_all(terminal::CHINESE.as_bytes())
        .unwrap();
    wait_for_screen(&fixture, terminal::ARGUMENT_PROMPT);
    fixture
}

pub(super) fn master_descriptor(fixture: &Fixture) -> std::os::fd::RawFd {
    lock_recover(&fixture.state.master)
        .as_ref()
        .unwrap()
        .as_raw_fd()
        .expect(MISSING_MASTER)
}

pub(super) fn reject_reader_construction(fixture: Fixture) -> std::io::Result<()> {
    let _owned_fixture = fixture;
    // Inject the reader-construction error result, without changing process limits.
    Err(std::io::Error::from_raw_os_error(libc::EMFILE))
}

pub(super) fn assert_reclaimed(state: &WorkerState, master: std::os::fd::RawFd) {
    assert!(lock_recover(&state.writer).is_none());
    assert!(lock_recover(&state.master).is_none());
    assert_eq!(
        unsafe { libc::fcntl(master, libc::F_GETFD) },
        SYSCALL_FAILURE
    );
    assert_eq!(
        std::io::Error::last_os_error().raw_os_error(),
        Some(libc::EBADF)
    );
    let root = state.pty_root_identity();
    assert_eq!(root.observe(), process::ProcessIdentityObservation::Gone);
    let mut status = 0;
    assert_eq!(
        unsafe { libc::waitpid(root.pid as libc::pid_t, &mut status, libc::WNOHANG) },
        SYSCALL_FAILURE
    );
    assert_eq!(
        std::io::Error::last_os_error().raw_os_error(),
        Some(libc::ECHILD)
    );
}

fn wait_for_screen(fixture: &Fixture, expected: &str) {
    let descriptor = master_descriptor(fixture);
    let deadline = Instant::now() + terminal::READ_TIMEOUT;
    let mut bytes = [0u8; terminal::READ_BYTES];
    loop {
        if lock_recover(&fixture.state.terminal)
            .screen()
            .contents()
            .contains(expected)
        {
            return;
        }
        assert!(Instant::now() < deadline, "{PROMPT_TIMEOUT}");
        let count = unsafe { libc::read(descriptor, bytes.as_mut_ptr().cast(), bytes.len()) };
        if count > 0 {
            lock_recover(&fixture.state.terminal).process(&bytes[..count as usize]);
        } else {
            assert_eq!(count, SYSCALL_FAILURE as libc::ssize_t);
            let error = std::io::Error::last_os_error();
            assert!(
                matches!(
                    error.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                ),
                "{error}"
            );
            std::thread::sleep(terminal::READ_POLL);
        }
    }
}
