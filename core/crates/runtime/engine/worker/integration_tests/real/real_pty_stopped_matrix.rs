const STOPPED_OUTPUT_ISOLATION_ENV: &str = "YOLO_REAL_PTY_STOPPED_ISOLATION";
const STOPPED_OUTPUT_ISOLATION_ROLE: &str = "isolated-worker";
const STOPPED_OUTPUT_CASE_ENV: &str = "YOLO_REAL_PTY_STOPPED_CASE";
const STOPPED_OUTPUT_ISOLATED_TEST: &str =
    "worker::tests::real_pty_exit::real_pty_stopped_output_isolated_worker";
const STOPPED_OUTPUT_ACTIVE_CTRL_S: &str = "active-force-ctrl-s";
const STOPPED_OUTPUT_ACTIVE_IOCTL: &str = "active-force-tiocstop";
const STOPPED_OUTPUT_FAULT_CTRL_S: &str = "deferred-fault-ctrl-s";
const STOPPED_OUTPUT_FAULT_IOCTL: &str = "deferred-fault-tiocstop";
const STOPPED_OUTPUT_STARTUP_CTRL_S: &str = "startup-failure-ctrl-s";
const STOPPED_OUTPUT_STARTUP_IOCTL: &str = "startup-failure-tiocstop";
const STOPPED_OUTPUT_UNKNOWN_CASE_DETAIL: &str = "unknown stopped-output test case";
static STOPPED_OUTPUT_ISOLATION_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum StoppedOutputMode {
    CtrlS,
    Tiocstop,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum StoppedOutputScenario {
    ActiveForce,
    DeferredFault,
    StartupFailure,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct StoppedOutputCase {
    scenario: StoppedOutputScenario,
    mode: StoppedOutputMode,
}

impl StoppedOutputCase {
    const fn name(self) -> &'static str {
        match (self.scenario, self.mode) {
            (StoppedOutputScenario::ActiveForce, StoppedOutputMode::CtrlS) => {
                STOPPED_OUTPUT_ACTIVE_CTRL_S
            }
            (StoppedOutputScenario::ActiveForce, StoppedOutputMode::Tiocstop) => {
                STOPPED_OUTPUT_ACTIVE_IOCTL
            }
            (StoppedOutputScenario::DeferredFault, StoppedOutputMode::CtrlS) => {
                STOPPED_OUTPUT_FAULT_CTRL_S
            }
            (StoppedOutputScenario::DeferredFault, StoppedOutputMode::Tiocstop) => {
                STOPPED_OUTPUT_FAULT_IOCTL
            }
            (StoppedOutputScenario::StartupFailure, StoppedOutputMode::CtrlS) => {
                STOPPED_OUTPUT_STARTUP_CTRL_S
            }
            (StoppedOutputScenario::StartupFailure, StoppedOutputMode::Tiocstop) => {
                STOPPED_OUTPUT_STARTUP_IOCTL
            }
        }
    }

    fn parse(name: &str) -> Result<Self> {
        STOPPED_OUTPUT_CASES
            .iter()
            .copied()
            .find(|case| case.name() == name)
            .ok_or_else(|| anyhow::anyhow!(STOPPED_OUTPUT_UNKNOWN_CASE_DETAIL))
    }
}

const STOPPED_OUTPUT_ACTIVE_CTRL_S_CASE: StoppedOutputCase = StoppedOutputCase {
    scenario: StoppedOutputScenario::ActiveForce,
    mode: StoppedOutputMode::CtrlS,
};
const STOPPED_OUTPUT_ACTIVE_IOCTL_CASE: StoppedOutputCase = StoppedOutputCase {
    scenario: StoppedOutputScenario::ActiveForce,
    mode: StoppedOutputMode::Tiocstop,
};
const STOPPED_OUTPUT_FAULT_CTRL_S_CASE: StoppedOutputCase = StoppedOutputCase {
    scenario: StoppedOutputScenario::DeferredFault,
    mode: StoppedOutputMode::CtrlS,
};
const STOPPED_OUTPUT_FAULT_IOCTL_CASE: StoppedOutputCase = StoppedOutputCase {
    scenario: StoppedOutputScenario::DeferredFault,
    mode: StoppedOutputMode::Tiocstop,
};
const STOPPED_OUTPUT_STARTUP_CTRL_S_CASE: StoppedOutputCase = StoppedOutputCase {
    scenario: StoppedOutputScenario::StartupFailure,
    mode: StoppedOutputMode::CtrlS,
};
const STOPPED_OUTPUT_STARTUP_IOCTL_CASE: StoppedOutputCase = StoppedOutputCase {
    scenario: StoppedOutputScenario::StartupFailure,
    mode: StoppedOutputMode::Tiocstop,
};
const STOPPED_OUTPUT_CASES: &[StoppedOutputCase] = &[
    STOPPED_OUTPUT_ACTIVE_CTRL_S_CASE,
    STOPPED_OUTPUT_ACTIVE_IOCTL_CASE,
    STOPPED_OUTPUT_FAULT_CTRL_S_CASE,
    STOPPED_OUTPUT_FAULT_IOCTL_CASE,
    STOPPED_OUTPUT_STARTUP_CTRL_S_CASE,
    STOPPED_OUTPUT_STARTUP_IOCTL_CASE,
];

#[test]
fn active_force_recovers_ctrl_s_backpressure() {
    run_stopped_output_isolation(STOPPED_OUTPUT_ACTIVE_CTRL_S_CASE).unwrap();
}

#[test]
fn active_force_recovers_tiocstop_backpressure() {
    run_stopped_output_isolation(STOPPED_OUTPUT_ACTIVE_IOCTL_CASE).unwrap();
}

#[test]
fn deferred_owner_fault_recovers_ctrl_s_backpressure() {
    run_stopped_output_isolation(STOPPED_OUTPUT_FAULT_CTRL_S_CASE).unwrap();
}

#[test]
fn deferred_owner_fault_recovers_tiocstop_backpressure() {
    run_stopped_output_isolation(STOPPED_OUTPUT_FAULT_IOCTL_CASE).unwrap();
}

#[test]
fn startup_failure_recovers_ctrl_s_backpressure() {
    run_stopped_output_isolation(STOPPED_OUTPUT_STARTUP_CTRL_S_CASE).unwrap();
}

#[test]
fn startup_failure_recovers_tiocstop_backpressure() {
    run_stopped_output_isolation(STOPPED_OUTPUT_STARTUP_IOCTL_CASE).unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn real_pty_stopped_output_isolated_worker() {
    if std::env::var(STOPPED_OUTPUT_ISOLATION_ENV).as_deref() != Ok(STOPPED_OUTPUT_ISOLATION_ROLE) {
        return;
    }
    let context = StoppedOutputContext::required().unwrap();
    context.publish_isolated_identity().unwrap();
    match context.case.scenario {
        StoppedOutputScenario::ActiveForce => run_stopped_active(&context).await,
        StoppedOutputScenario::DeferredFault => run_stopped_fault(&context).await,
        StoppedOutputScenario::StartupFailure => run_stopped_startup(&context),
    }
    context.complete().unwrap();
}

include!("real_pty_stopped_isolation.rs");
include!("../../test_support/integration/real_pty/real_pty_stopped_terminal.rs");
include!("real_pty_stopped_writer.rs");
include!("../../test_support/integration/real_pty/real_pty_stopped_transcript.rs");
include!("real_pty_stopped_active.rs");
include!("../../test_support/integration/real_pty/real_pty_stopped_fault.rs");
include!("../../test_support/integration/real_pty/real_pty_stopped_startup.rs");
