use std::io::Write;
use std::process::{Command, Output};

const GATE_EXECUTABLE_ENV: &str = "YOLO_COVERAGE_GATE_EXECUTABLE";

pub fn execute(report: &str, scope: Option<&str>) -> Output {
    let mut file = tempfile::NamedTempFile::new().unwrap();
    file.write_all(report.as_bytes()).unwrap();
    let mut command = Command::new(std::env::var_os(GATE_EXECUTABLE_ENV).unwrap());
    if let Some(scope) = scope {
        command.arg(scope);
    }
    command.arg(file.path()).output().unwrap()
}

pub fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).unwrap()
}
