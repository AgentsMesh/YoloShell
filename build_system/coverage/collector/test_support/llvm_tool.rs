#[allow(dead_code)]
mod constants;

use constants::*;
use std::io::Write;
use std::path::PathBuf;

const LCOV_ARGUMENT: &str = "-format=lcov";
const COMPLETION_ENV: &str = "TEST_PREMATURE_EXIT_FILE";
const INVALID_UTF8: &[u8] = &[0xff];

fn main() {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let stage = match args.first().map(String::as_str) {
        Some(MERGE_STAGE) => MERGE_STAGE,
        Some(REPORT_STAGE) => REPORT_STAGE,
        Some(FIXTURE_EXPORT_COMMAND) if args.iter().any(|arg| arg == LCOV_ARGUMENT) => LCOV_STAGE,
        Some(FIXTURE_EXPORT_COMMAND) => JSON_STAGE,
        other => panic!("unexpected LLVM fixture command {other:?}"),
    };
    let failure = std::env::var(FIXTURE_FAILURE_ENV).unwrap_or_default();
    if failure == stage {
        eprintln!("{FIXTURE_FAILURE_MESSAGE}");
        std::process::exit(TOOL_FAILURE_CODE);
    }
    if stage == MERGE_STAGE {
        if failure != CLEANUP_FAILURE {
            let output = args
                .windows(2)
                .find(|pair| pair[0] == FIXTURE_OUTPUT_ARGUMENT)
                .unwrap();
            std::fs::write(&output[1], MERGE_STAGE).unwrap();
        }
        return;
    }
    if failure == UTF8_FAILURE {
        std::io::stdout().write_all(INVALID_UTF8).unwrap();
        return;
    }
    if failure == MARKER_FAILURE {
        std::fs::write(std::env::var_os(COMPLETION_ENV).unwrap(), MARKER_FAILURE).unwrap();
    }
    let name = match stage {
        LCOV_STAGE => LCOV_FILE,
        JSON_STAGE => JSON_FILE,
        REPORT_STAGE => REPORT_FILE,
        _ => unreachable!(),
    };
    let directory = PathBuf::from(std::env::var_os(FIXTURE_DIRECTORY_ENV).unwrap());
    std::io::stdout()
        .write_all(&std::fs::read(directory.join(name)).unwrap())
        .unwrap();
}
