use std::io::Write;
use std::path::Path;
use yolo_codex_version_fixture_contract as contract;

const VERSION_ARGUMENT: &str = env!("YOLO_CODEX_VERSION_ARGUMENT");
const ACCEPTED_MODE: &str = "accepted";
const WRONG_MODE: &str = "wrong";
const HUNG_MODE: &str = "hung";
const OVERSIZED_MODE: &str = "oversized";
const REQUIRES_CHILD_MODE: &str = "requires-child";
const LIMITS_MODE: &str = "limits";
const ACCEPTED_OUTPUT: &[u8] = b"codex-cli 0.150.1\n";
const WRONG_OUTPUT: &[u8] = b"codex-cli 0.150.0\n";
const OVERSIZED_CHUNK_BYTES: usize = 256;
const SINGLE_PROCESS_LIMIT: libc::rlim_t = 1;
const RAISED_PROCESS_LIMIT: libc::rlim_t = 2;
const SOFT_LIMIT_FILE: &str = "soft-limit";
const HARD_LIMIT_FILE: &str = "hard-limit";
const TRUE_EXECUTABLE: &str = "/usr/bin/true";
const INVALID_ARGUMENT_EXIT: i32 = 70;
const UNKNOWN_MODE_EXIT: i32 = 71;
const CHILD_REQUIRED_EXIT: i32 = 72;
const LIMIT_INVALID_EXIT: i32 = 73;
const LIMIT_RAISED_EXIT: i32 = 74;
const SYSTEM_CALL_FAILURE: libc::c_int = -1;
const EXECUTABLE_ARGUMENT_INDEX: usize = 0;
const VERSION_ARGUMENT_INDEX: usize = 1;
const EXPECTED_ARGUMENT_COUNT: usize = 2;

fn main() {
    let arguments = std::env::args_os().collect::<Vec<_>>();
    if arguments.len() != EXPECTED_ARGUMENT_COUNT
        || arguments[VERSION_ARGUMENT_INDEX] != VERSION_ARGUMENT
    {
        std::process::exit(INVALID_ARGUMENT_EXIT);
    }
    let executable = Path::new(&arguments[EXECUTABLE_ARGUMENT_INDEX]);
    record_resume(executable);
    let mode = executable.file_name().and_then(|value| value.to_str());
    match mode {
        Some(ACCEPTED_MODE) => write_output(ACCEPTED_OUTPUT),
        Some(WRONG_MODE) => write_output(WRONG_OUTPUT),
        Some(HUNG_MODE) => loop {
            std::thread::park();
        },
        Some(OVERSIZED_MODE) => oversized_output(),
        Some(REQUIRES_CHILD_MODE) => require_child(),
        Some(LIMITS_MODE) => verify_limits(executable),
        _ => std::process::exit(UNKNOWN_MODE_EXIT),
    }
}

fn record_resume(executable: &Path) {
    let directory = executable.parent().unwrap();
    if directory.join(contract::RESUME_REQUEST_FILE).is_file() {
        std::fs::write(
            directory.join(contract::RESUME_MARKER_FILE),
            contract::RESUME_MARKER_CONTENT,
        )
        .unwrap();
    }
}

fn write_output(output: &[u8]) {
    std::io::stdout().write_all(output).unwrap();
}

fn oversized_output() {
    let chunk = [b'x'; OVERSIZED_CHUNK_BYTES];
    while std::io::stdout().write_all(&chunk).is_ok() {}
}

fn require_child() {
    if std::process::Command::new(TRUE_EXECUTABLE)
        .status()
        .is_err()
    {
        std::process::exit(CHILD_REQUIRED_EXIT);
    }
    write_output(ACCEPTED_OUTPUT);
}

fn verify_limits(executable: &Path) {
    let mut limit = std::mem::MaybeUninit::<libc::rlimit>::uninit();
    if unsafe { libc::getrlimit(libc::RLIMIT_NPROC, limit.as_mut_ptr()) } == SYSTEM_CALL_FAILURE {
        std::process::exit(LIMIT_INVALID_EXIT);
    }
    let limit = unsafe { limit.assume_init() };
    if limit.rlim_cur != SINGLE_PROCESS_LIMIT || limit.rlim_max != SINGLE_PROCESS_LIMIT {
        std::process::exit(LIMIT_INVALID_EXIT);
    }
    let directory = executable.parent().unwrap();
    std::fs::write(directory.join(SOFT_LIMIT_FILE), limit.rlim_cur.to_string()).unwrap();
    std::fs::write(directory.join(HARD_LIMIT_FILE), limit.rlim_max.to_string()).unwrap();
    let raised = libc::rlimit {
        rlim_cur: RAISED_PROCESS_LIMIT,
        rlim_max: RAISED_PROCESS_LIMIT,
    };
    if unsafe { libc::setrlimit(libc::RLIMIT_NPROC, &raised) } != SYSTEM_CALL_FAILURE {
        std::process::exit(LIMIT_RAISED_EXIT);
    }
    write_output(ACCEPTED_OUTPUT);
}
