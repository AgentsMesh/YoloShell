impl Fixture {
    fn start_termination_runtime(&self) -> Option<FixtureTerminationRuntime> {
        if lock_recover(&self.startup).is_none() {
            self.state.reader_lifecycle.mark_drained();
            self.start_child_waiter();
            return None;
        }
        let reader = lock_recover(&self.state.master)
            .as_ref()
            .unwrap()
            .try_clone_reader()
            .unwrap();
        let mut runtime = FixtureTerminationRuntime::prepare(Arc::clone(&self.state));
        let mut guard = lock_recover(&self.startup).take().unwrap();
        runtime.release = Some(FixtureHostRelease::respond_on(self, &runtime.executor));
        guard
            .activate_reader(
                pty_reader_activation::stage_reader(reader, Arc::clone(&self.state)).unwrap(),
            )
            .unwrap();
        let staged =
            pty_owner_handle::stage_guarded_owner(&mut guard, Arc::clone(&self.state)).unwrap();
        runtime.settlement = Some(staged.settlement_for_test());
        staged.activate(&mut guard).unwrap();
        Some(runtime)
    }
}

include!("fixture_termination_runtime.rs");
include!("fixture_termination_cleanup.rs");

#[cfg(all(feature = "yolo-unit-tests", feature = "yolo-worker-unit-runtime", target_os = "linux"))]
#[path = "../../../unit_tests/fixture_termination_failure_tests.rs"]
mod fixture_termination_failure_tests;
