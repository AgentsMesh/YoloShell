const FINALIZER_DELAYED_CONTROLLER_SCENARIO: &str = "delayed-controller-fencing";
const FINALIZER_DELAYED_CONTROLLER_TEST: &str = "worker::tests::finalization_barrier::delayed_finalizer_controller_shutdown_uses_process_deadline";

#[derive(Clone, Copy)]
enum FinalizerProcessScenario {
    ResourceMutations,
    ControllerFencing,
    DelayedControllerFencing,
    AuthorityUpgrade,
}

impl FinalizerProcessScenario {
    fn environment_value(self) -> &'static str {
        match self {
            Self::ResourceMutations => FINALIZER_RESOURCE_SCENARIO,
            Self::ControllerFencing => FINALIZER_CONTROLLER_SCENARIO,
            Self::DelayedControllerFencing => FINALIZER_DELAYED_CONTROLLER_SCENARIO,
            Self::AuthorityUpgrade => FINALIZER_AUTHORITY_SCENARIO,
        }
    }

    fn test_name(self) -> &'static str {
        match self {
            Self::ResourceMutations => FINALIZER_RESOURCE_TEST,
            Self::ControllerFencing => FINALIZER_CONTROLLER_TEST,
            Self::DelayedControllerFencing => FINALIZER_DELAYED_CONTROLLER_TEST,
            Self::AuthorityUpgrade => FINALIZER_AUTHORITY_TEST,
        }
    }

    fn is_child(self) -> bool {
        let Some(value) = std::env::var_os(FINALIZER_PROCESS_SCENARIO_ENVIRONMENT) else {
            return false;
        };
        assert_eq!(
            value.to_string_lossy(),
            self.environment_value(),
            "{FINALIZER_PROCESS_SCENARIO_DETAIL}"
        );
        wait_for_finalizer_start_gate();
        acknowledge_finalizer_child();
        true
    }
}
