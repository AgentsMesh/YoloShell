#[path = "../test_support/commands.rs"]
mod commands;
#[path = "../test_support/constants.rs"]
mod constants;
#[path = "../test_support/report.rs"]
mod report;
#[path = "../test_support/resources.rs"]
mod resources;
use constants::*;
use report::Exported;
use std::{
    collections::BTreeSet,
    ffi::OsStr,
    os::unix::process::ExitStatusExt,
    path::Path,
    process::{Command, Stdio},
};
use tokio::io::AsyncReadExt;
use yolo_process::ScopedWaitableChild;

#[tokio::test]
async fn continuous_profile_records_natural_exit() {
    verify(PROBE_ENV, &[NORMAL], true).await;
}

#[tokio::test]
async fn continuous_profile_keeps_exact_counts_after_sigkill() {
    verify(PROBE_ENV, &[BLOCKED], true).await;
}

#[tokio::test]
async fn pipeline_profile_matches_normal_build_or_coverage_after_exit() {
    verify(
        PIPELINE_PROBE_ENV,
        &[NORMAL],
        resources::settings()[COVERAGE_FIELD].as_bool().unwrap(),
    )
    .await;
}

#[tokio::test]
async fn pipeline_profile_matches_normal_build_or_coverage_after_sigkill() {
    verify(
        PIPELINE_PROBE_ENV,
        &[BLOCKED],
        resources::settings()[COVERAGE_FIELD].as_bool().unwrap(),
    )
    .await;
}

#[tokio::test]
async fn mixed_exit_churn_keeps_every_executed_counter_without_marking_cold_code() {
    let modes = (0..CHURN_REPETITIONS)
        .flat_map(|_| [NORMAL, BLOCKED])
        .collect::<Vec<_>>();
    verify(PROBE_ENV, &modes, true).await;
}

#[tokio::test]
async fn concurrent_counters_survive_sigkill_without_lost_updates() {
    verify(PROBE_ENV, &[THREADED], true).await;
}

#[tokio::test]
async fn interruption_inside_a_loop_preserves_valid_region_and_branch_counts() {
    verify(PROBE_ENV, &[IN_LOOP], true).await;
}

#[tokio::test]
async fn unwound_loop_preserves_valid_region_and_branch_counts() {
    verify(PROBE_ENV, &[PANIC_LOOP], true).await;
}

#[tokio::test]
async fn early_return_loop_preserves_valid_region_and_branch_counts() {
    verify(PROBE_ENV, &[RETURN_LOOP], true).await;
}

#[tokio::test]
async fn error_before_unvisited_loop_keeps_region_expressions_valid() {
    verify(PROBE_ENV, &[BEFORE_LOOP], true).await;
}

#[tokio::test]
async fn interrupted_result_before_loop_keeps_region_expressions_valid() {
    verify(PROBE_ENV, &[HUNG_RESULT], true).await;
}

async fn execute_probe(binary: &Path, mode: &str, directory: &Path) -> (u32, Vec<u8>) {
    let mut command = Command::new(&binary);
    command
        .arg(mode)
        .env(PROFILE_ENV, directory.join(PROFILE_PATTERN))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let limit = tokio::time::Instant::now() + TIMEOUT;
    let mut child =
        ScopedWaitableChild::spawn_before_deadline(&mut command, limit.into_std()).unwrap();
    let pid = child.process_id();
    drop(command);
    let mut stdout = tokio::process::ChildStdout::from_std(child.take_stdout().unwrap()).unwrap();
    let stderr = tokio::process::ChildStderr::from_std(child.take_stderr().unwrap()).unwrap();
    let run = async {
        let mut ready = [0; READY.len()];
        stdout.read_exact(&mut ready).await.unwrap();
        assert_eq!(&ready, READY);
        if matches!(
            mode,
            BLOCKED | THREADED | IN_LOOP | PANIC_LOOP | RETURN_LOOP | BEFORE_LOOP | HUNG_RESULT
        ) {
            let status = child
                .terminate_and_wait_status(limit.into_std())
                .await
                .unwrap();
            assert_eq!(status.signal(), Some(libc::SIGKILL));
        } else {
            child.wait_for_clean_exit(limit.into_std()).await.unwrap();
        }
    };
    let (_, errors) =
        tokio::time::timeout_at(limit, async { tokio::join!(run, commands::read(stderr)) })
            .await
            .unwrap();
    (pid, errors)
}

async fn verify(variable: &str, modes: &[&str], instrumented: bool) {
    let directory = tempfile::tempdir().unwrap();
    let binary = resources::binary(variable);
    let mut processes = BTreeSet::new();
    let mut errors = Vec::new();
    for mode in modes {
        let (pid, diagnostic) = execute_probe(&binary, mode, directory.path()).await;
        processes.insert(pid);
        errors.extend(diagnostic);
    }
    let raw = std::fs::read_dir(directory.path())
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension() == Some(OsStr::new(RAW_EXTENSION)))
        .collect::<Vec<_>>();
    if !instrumented {
        assert!(raw.is_empty());
        assert!(errors.is_empty());
        return;
    }
    assert_eq!(
        raw.len(),
        processes.len(),
        "{}",
        String::from_utf8_lossy(&errors)
    );
    let indexed = directory.path().join(INDEX_FILE);
    let mut merge = Command::new(resources::tool(PROFDATA_FIELD));
    merge
        .arg(MERGE)
        .arg(SPARSE)
        .args(raw)
        .arg(OUTPUT)
        .arg(&indexed);
    commands::output(merge).await;
    let mut export = Command::new(resources::tool(COV_FIELD));
    export
        .arg(EXPORT)
        .arg(JSON_FORMAT)
        .arg(PROFILE_ARGUMENT)
        .arg(indexed)
        .arg(binary);
    let report: Exported = serde_json::from_slice(&commands::output(export).await).unwrap();
    report.assert_finite_counts();
    let observed = modes
        .iter()
        .map(|mode| {
            if *mode == THREADED {
                (THREAD_COUNT * CALLS_PER_THREAD) as u64
            } else {
                EXECUTED
            }
        })
        .sum::<u64>();
    for (name, expected) in [(OBSERVED, observed), (UNOBSERVED, NOT_EXECUTED)] {
        let counts = report
            .data
            .iter()
            .flat_map(|data| &data.functions)
            .filter(|function| function.name == name)
            .map(|function| function.count)
            .collect::<Vec<_>>();
        assert_eq!(counts.len(), SINGLE_FUNCTION, "{name}");
        assert_eq!(
            counts,
            vec![expected],
            "{name}: {}",
            String::from_utf8_lossy(&errors)
        );
    }
}
