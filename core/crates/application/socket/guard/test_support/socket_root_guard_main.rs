use std::process::ExitCode;

#[path = "mod.rs"]
mod socket_root_guard;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-test-support"))]
use yolo_application_test_fixtures::executables as test_executables;

use socket_root_guard::constants::ERROR_PREFIX;

fn main() -> ExitCode {
    match socket_root_guard::run() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("{ERROR_PREFIX}: {error:#}");
            ExitCode::FAILURE
        }
    }
}
