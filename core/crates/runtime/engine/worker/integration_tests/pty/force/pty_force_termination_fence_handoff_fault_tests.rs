const FENCE_POLL_ERROR_DETAIL: &str = "injected retained-fence poll error";
const FENCE_POLL_PANIC_DETAIL: &str = "injected retained-fence poll panic";
const EOF_COMMIT_ERROR_DETAIL: &str = "injected retained EOF commit error";
const EOF_COMMIT_PANIC_DETAIL: &str = "injected retained EOF commit panic";
const EXPECTED_FORCE_FAULT_DETAIL: &str = "force transaction fault was not observed";
const FORCE_RETRY_KILLING_DETAIL: &str = "retained force transaction did not enter killing";
const RETRIED_EOF_TOKEN_MINT_COUNT: usize = OPERATION_CALL_INCREMENT * 2;

#[derive(Clone, Copy)]
enum RetainedForceFault {
    FencePollError,
    FencePollPanic,
    EofCommitError,
    EofCommitPanic,
}

#[test]
fn fence_poll_error_handoff_resumes_same_kernel_eof_force() {
    assert_fault_handoff_resumes(RetainedForceFault::FencePollError);
}

#[test]
fn fence_poll_panic_handoff_resumes_same_kernel_eof_force() {
    assert_fault_handoff_resumes(RetainedForceFault::FencePollPanic);
}

#[test]
fn eof_commit_error_handoff_resumes_same_kernel_eof_force() {
    assert_fault_handoff_resumes(RetainedForceFault::EofCommitError);
}

#[test]
fn eof_commit_panic_handoff_resumes_same_kernel_eof_force() {
    assert_fault_handoff_resumes(RetainedForceFault::EofCommitPanic);
}

fn assert_fault_handoff_resumes(fault: RetainedForceFault) {
    let (mut ownership, held, mut output, version) =
        pending_scripted_kernel_eof_fixture(fault.fence_behavior());
    fault.inject(&mut output);
    fault.trigger(&mut ownership, &mut output);

    let (completion, receiver) = FaultReapCompletion::owner_channel();
    assert!(matches!(
        ownership.settle_after_fault(completion),
        FaultSettlement::Deferred(_)
    ));
    let mut handoff = held.wait_for_handoff();
    handoff.with_force_transaction_for_test(|force, custody| {
        assert_scripted_force_transaction(force, custody, &mut output, &version, fault);
        output.set_fence(FenceBehavior::Reached);
        assert_pending(force.poll(custody, &mut ForceOutput::Scripted(&mut output)));
        assert_eq!(
            custody.as_ref().unwrap().snapshot_for_test().kind,
            PtyProcessCustodyKind::Killing,
            "{FORCE_RETRY_KILLING_DETAIL}"
        );
        assert!(force.output_fence.is_none());
    });
    assert_eq!(output.eof_acquire_calls, OPERATION_CALL_INCREMENT);
    assert_eq!(output.begin_calls, usize::default());
    assert_eq!(output.final_eof_token_mints, fault.expected_token_mints());
    handoff.resume();
    let outcome = receiver
        .receive_timeout(HANDOFF_COMPLETION_TIMEOUT)
        .unwrap();
    assert_eq!(outcome.evidence, ReapEvidence::Waited);
    assert!(outcome.status.is_some());
}

fn assert_scripted_force_transaction(
    force: &PtyForceTermination,
    custody: &mut Option<PtyProcessCustody>,
    output: &mut ScriptedForceOutput,
    version: &ProcessClosureVersion,
    fault: RetainedForceFault,
) {
    assert_eq!(
        custody.as_ref().unwrap().snapshot_for_test().kind,
        PtyProcessCustodyKind::Frozen,
        "{FORCE_HANDOFF_FROZEN_DETAIL}"
    );
    let fence = force
        .output_fence
        .as_ref()
        .expect(FORCE_HANDOFF_FENCE_DETAIL);
    assert_eq!(&fence.version, version);
    assert!(fault.matches_retained_fence(&fence.state));
    let authority = fence
        .kernel_eof
        .as_ref()
        .expect(FORCE_HANDOFF_KERNEL_EOF_DETAIL);
    assert!(
        output
            .validates_kernel_eof_authority(authority, version)
            .unwrap()
    );
}

impl RetainedForceFault {
    fn fence_behavior(self) -> FenceBehavior {
        match self {
            Self::FencePollError => FenceBehavior::PollError(FENCE_POLL_ERROR_DETAIL),
            Self::FencePollPanic => FenceBehavior::Pending,
            Self::EofCommitError | Self::EofCommitPanic => FenceBehavior::Reached,
        }
    }

    fn inject(self, output: &mut ScriptedForceOutput) {
        match self {
            Self::FencePollError => {}
            Self::FencePollPanic => output.panic_on_next_fence_poll(FENCE_POLL_PANIC_DETAIL),
            Self::EofCommitError => output.fail_after_final_eof_token_mint(EOF_COMMIT_ERROR_DETAIL),
            Self::EofCommitPanic => {
                output.panic_after_final_eof_token_mint(EOF_COMMIT_PANIC_DETAIL)
            }
        }
    }

    fn trigger(self, ownership: &mut PtyOwnership, output: &mut ScriptedForceOutput) {
        let mut poll = || ownership.poll_force(&mut ForceOutput::Scripted(output));
        match self {
            Self::FencePollError | Self::EofCommitError => {
                let error = match poll() {
                    Err(error) => error,
                    Ok(_) => panic!("{EXPECTED_FORCE_FAULT_DETAIL}"),
                };
                assert!(format!("{error:#}").contains(self.detail()));
            }
            Self::FencePollPanic | Self::EofCommitPanic => {
                let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(poll));
                assert!(panic.is_err(), "{EXPECTED_FORCE_FAULT_DETAIL}");
            }
        }
    }

    fn matches_retained_fence(self, fence: &OutputFence) -> bool {
        match self {
            Self::FencePollError | Self::FencePollPanic => matches!(fence, OutputFence::Scripted),
            Self::EofCommitError | Self::EofCommitPanic => matches!(fence, OutputFence::Reached),
        }
    }

    fn detail(self) -> &'static str {
        match self {
            Self::FencePollError => FENCE_POLL_ERROR_DETAIL,
            Self::FencePollPanic => FENCE_POLL_PANIC_DETAIL,
            Self::EofCommitError => EOF_COMMIT_ERROR_DETAIL,
            Self::EofCommitPanic => EOF_COMMIT_PANIC_DETAIL,
        }
    }

    fn expected_token_mints(self) -> usize {
        match self {
            Self::FencePollError | Self::FencePollPanic => OPERATION_CALL_INCREMENT,
            Self::EofCommitError | Self::EofCommitPanic => RETRIED_EOF_TOKEN_MINT_COUNT,
        }
    }
}
