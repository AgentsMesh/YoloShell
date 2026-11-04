const PREPARED_GONE_REQUIRED_DETAIL: &str = "kernel EOF did not prepare a Gone transition";
const STALE_KERNEL_EOF_RETAINED_DETAIL: &str =
    "late process authority loss retained the stale kernel EOF";

#[test]
fn note_fork_after_prepare_cannot_commit_gone() {
    let (_source, source_tree) = ChildGuard::spawn();
    let source = source_tree.root();
    let mut fixture = PreparedGoneFixture::new();
    fixture
        .frozen_tree()
        .inject_unresolved_fork_for_test(source);

    fixture.assert_commit_refreezes();
}

#[test]
fn event_gate_failure_after_prepare_cannot_commit_gone() {
    let mut fixture = PreparedGoneFixture::new();
    fixture.frozen_tree().fail_next_event_read_for_test();

    fixture.assert_commit_refreezes();
}

#[test]
fn no_slave_producer_after_prepare_cannot_commit_gone() {
    let (_producer, producer_tree) = ChildGuard::spawn();
    let producer = producer_tree.root();
    let mut fixture = PreparedGoneFixture::new();
    fixture
        .frozen_tree()
        .inject_captured_producer_for_test(producer);

    fixture.assert_commit_refreezes();
}

struct PreparedGoneFixture {
    _child: ChildGuard,
    custody: Option<PtyProcessCustody>,
    force: PtyForceTermination,
    output: ScriptedForceOutput,
    ready: Option<ForceReady>,
    root: ProcessIdentity,
}

impl PreparedGoneFixture {
    fn new() -> Self {
        let (child, tree) = exited_empty_closure_fixture();
        let root = tree.root();
        let mut custody = Some(PtyProcessCustody::tracking(tree));
        let mut force = PtyForceTermination::requiring_empty_closure_output_for_test(
            EmptyClosureOutputRequirement::KernelEof,
        );
        let mut output = ScriptedForceOutput::new(FenceBehavior::Pending);
        drive_to_frozen(&mut force, &mut custody, &mut output);
        output.observe_kernel_eof();
        let ready = prepare_force_without_commit(&mut force, &mut custody, &mut output);
        assert!(
            matches!(ready, ForceReady::FrozenGone),
            "{PREPARED_GONE_REQUIRED_DETAIL}"
        );
        Self {
            _child: child,
            custody,
            force,
            output,
            ready: Some(ready),
            root,
        }
    }

    fn frozen_tree(&mut self) -> &mut FrozenProcessTree {
        frozen_tree_mut(&mut self.custody)
    }

    fn assert_commit_refreezes(&mut self) {
        let ready = self.ready.take().unwrap();
        assert_pending(commit_prepared_force(
            &mut self.force,
            &mut self.custody,
            &mut self.output,
            ready,
        ));
        self.output
            .assert_kernel_eof_revoked(STALE_KERNEL_EOF_RETAINED_DETAIL);
        assert_variant_root(&self.custody, CustodyKind::Frozen, self.root);
        assert_pending(poll_force(
            &mut self.force,
            &mut self.custody,
            &mut self.output,
        ));
        assert_variant_root(&self.custody, CustodyKind::Freezing, self.root);
    }
}
