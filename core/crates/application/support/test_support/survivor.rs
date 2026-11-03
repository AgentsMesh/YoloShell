use std::io;

const BOOTSTRAP_FAILURE_ENVIRONMENT: &str = "YOLO_TEST_SURVIVOR_BOOTSTRAP_FAILURE";
const BOOTSTRAP_FAILURE_DETAIL: &str = "injected survivor bootstrap failure";

pub fn bootstrap_failure_enabled() -> bool {
    std::env::var_os(BOOTSTRAP_FAILURE_ENVIRONMENT).is_some()
}

pub fn bootstrap_failure() -> io::Error {
    io::Error::other(BOOTSTRAP_FAILURE_DETAIL)
}
