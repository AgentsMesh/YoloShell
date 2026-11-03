use super::*;

const TEST_DEADLINE: Duration = Duration::from_secs(3);
const BEFORE_DEADLINE: Duration = Duration::from_secs(1);

struct FakeCustody {
    reported: bool,
    retired: bool,
}

struct FakeOutput {
    progress: OutputProgressState,
    permits_abandonment: bool,
}

impl MissingEofCustody for FakeCustody {
    fn producers_are_reported_gone(&self) -> bool {
        self.reported
    }

    fn retire_abandoned_output(&mut self, _proof: AbandonedPtyOutput) -> Result<()> {
        self.retired = true;
        Ok(())
    }
}

impl MissingEofOutput for FakeOutput {
    fn missing_eof_progress(&mut self) -> Result<OutputProgressState> {
        Ok(self.progress)
    }

    fn settle_missing_eof(&mut self, proof: MissingEofExpired) -> Result<OutputAbandonment> {
        Ok(if self.permits_abandonment {
            OutputAbandonment::Abandoned(proof.abandon())
        } else {
            OutputAbandonment::Retry
        })
    }
}

fn custody(reported: bool) -> FakeCustody {
    FakeCustody {
        reported,
        retired: false,
    }
}

fn output(progress: OutputProgressState) -> FakeOutput {
    FakeOutput {
        progress,
        permits_abandonment: true,
    }
}

#[test]
fn deadline_cannot_start_before_producers_are_reported_gone() {
    let mut deadline = MissingEofDeadline::with_timeout(Duration::ZERO);
    let mut custody = custody(false);
    let mut output = output(OutputProgressState::Quiescent);

    assert_eq!(
        poll(&mut deadline, &mut custody, &mut output, Instant::now()).unwrap(),
        MissingEofPoll::Pending
    );
    assert!(!custody.retired);
}

#[test]
fn holder_eof_before_deadline_preserves_normal_completion() {
    let started = Instant::now();
    let mut deadline = MissingEofDeadline::with_timeout(TEST_DEADLINE);
    let mut custody = custody(true);
    let mut output = output(OutputProgressState::Quiescent);
    assert_eq!(
        poll(&mut deadline, &mut custody, &mut output, started).unwrap(),
        MissingEofPoll::Pending
    );

    output.progress = OutputProgressState::Complete;
    assert_eq!(
        poll(
            &mut deadline,
            &mut custody,
            &mut output,
            started + BEFORE_DEADLINE,
        )
        .unwrap(),
        MissingEofPoll::Pending
    );
    assert!(!custody.retired);
    assert!(deadline.expires_at.is_none());
}

#[test]
fn pending_output_resets_the_missing_eof_deadline() {
    let started = Instant::now();
    let first_expiry = started + TEST_DEADLINE;
    let mut deadline = MissingEofDeadline::with_timeout(TEST_DEADLINE);
    let mut custody = custody(true);
    let mut output = output(OutputProgressState::Quiescent);
    poll(&mut deadline, &mut custody, &mut output, started).unwrap();

    output.progress = OutputProgressState::Pending;
    poll(&mut deadline, &mut custody, &mut output, first_expiry).unwrap();
    output.progress = OutputProgressState::Quiescent;
    assert_eq!(
        poll(&mut deadline, &mut custody, &mut output, first_expiry).unwrap(),
        MissingEofPoll::Pending
    );
    assert_eq!(
        poll(
            &mut deadline,
            &mut custody,
            &mut output,
            first_expiry + TEST_DEADLINE,
        )
        .unwrap(),
        MissingEofPoll::Lost
    );
    assert!(custody.retired);
}

#[test]
fn atomic_recheck_retry_restarts_the_full_deadline() {
    let started = Instant::now();
    let first_expiry = started + TEST_DEADLINE;
    let mut deadline = MissingEofDeadline::with_timeout(TEST_DEADLINE);
    let mut custody = custody(true);
    let mut output = output(OutputProgressState::Quiescent);
    poll(&mut deadline, &mut custody, &mut output, started).unwrap();
    output.permits_abandonment = false;
    poll(&mut deadline, &mut custody, &mut output, first_expiry).unwrap();

    output.permits_abandonment = true;
    assert_eq!(
        poll(&mut deadline, &mut custody, &mut output, first_expiry).unwrap(),
        MissingEofPoll::Pending
    );
    assert!(!custody.retired);
}

#[test]
fn zero_deadline_returns_typed_lost_settlement_without_sleeping() {
    let mut deadline = MissingEofDeadline::with_timeout(Duration::ZERO);
    let mut custody = custody(true);
    let mut output = output(OutputProgressState::Quiescent);

    assert_eq!(
        poll(&mut deadline, &mut custody, &mut output, Instant::now()).unwrap(),
        MissingEofPoll::Lost
    );
    assert!(custody.retired);
}

#[test]
fn exclusive_abandonment_persists_across_force_polls() {
    let observed = std::sync::Arc::new(std::sync::atomic::AtomicUsize::default());
    let mut output =
        crate::worker::pty_output_progress::ExclusivePtyOutput::quiescent_for_test(observed);
    let expired = MissingEofDeadline::with_timeout(Duration::ZERO)
        .observe(OutputProgressState::Quiescent, Instant::now())
        .unwrap()
        .unwrap();

    assert!(matches!(
        output.settle_missing_eof(expired).unwrap(),
        OutputAbandonment::Abandoned(_)
    ));
    assert_eq!(
        output.missing_eof_progress().unwrap(),
        OutputProgressState::Abandoned
    );
}
