use super::*;
use std::cell::Cell;

const MATRIX_FD: RawFd = SYSCALL_FAILURE;

#[derive(Clone, Copy, Debug)]
enum PollCase {
    Cancelled,
    Probe,
    Ready,
    InterruptedThenReady,
    Failure,
    TimeoutThenCancel,
    Unexpected,
}

#[test]
fn one_poll_instance_covers_every_readiness_transition() {
    for case in [
        PollCase::Cancelled,
        PollCase::Probe,
        PollCase::Ready,
        PollCase::InterruptedThenReady,
        PollCase::Failure,
        PollCase::TimeoutThenCancel,
        PollCase::Unexpected,
    ] {
        assert_poll_case(case);
    }
}

fn assert_poll_case(case: PollCase) {
    let cancelled = AtomicBool::new(matches!(case, PollCase::Cancelled));
    let probe = AtomicBool::new(matches!(case, PollCase::Probe));
    let interrupted = Cell::new(false);
    let result =
        wait_for_pty_readiness_with(
            MATRIX_FD,
            &cancelled,
            &probe,
            |descriptor, flag| match case {
                PollCase::Ready => ready(descriptor),
                PollCase::InterruptedThenReady if !interrupted.replace(true) => {
                    set_errno(libc::EINTR);
                    SYSCALL_FAILURE
                }
                PollCase::InterruptedThenReady => ready(descriptor),
                PollCase::Failure => {
                    set_errno(libc::EBADF);
                    SYSCALL_FAILURE
                }
                PollCase::TimeoutThenCancel => {
                    flag.store(true, Ordering::Release);
                    POLL_TIMEOUT_RESULT
                }
                PollCase::Unexpected => UNEXPECTED_POLL_RESULT,
                PollCase::Cancelled | PollCase::Probe => unreachable!(),
            },
        );

    match case {
        PollCase::Cancelled | PollCase::TimeoutThenCancel => {
            assert_eq!(result.unwrap(), PtyReadiness::Cancelled)
        }
        PollCase::Probe => assert_eq!(result.unwrap(), PtyReadiness::Probe),
        PollCase::Ready | PollCase::InterruptedThenReady => {
            assert_eq!(result.unwrap(), PtyReadiness::Readable)
        }
        PollCase::Failure | PollCase::Unexpected => assert!(result.is_err()),
    }
}

fn ready(descriptor: &mut libc::pollfd) -> i32 {
    descriptor.revents = libc::POLLIN;
    POLL_READY_DESCRIPTOR_COUNT
}
