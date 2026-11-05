use super::*;

#[test]
fn bootstrap_gate_child_process_integration() {
    if std::env::var_os(BOOTSTRAP_GATE_SPEC_ENV).is_none() {
        return;
    }
    let result = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(crate::worker::run_bootstrap_child());
    if result.is_err() {
        std::process::exit(BOOTSTRAP_TEST_FAILURE_EXIT_CODE);
    }
}
