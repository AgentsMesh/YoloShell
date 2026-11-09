use crate::{commands, constants::*, resources};
use serde_json::Value;
use std::{
    ffi::OsStr,
    os::unix::process::ExitStatusExt,
    path::Path,
    process::{Command, Stdio},
};
use tokio::io::AsyncReadExt;
use yolo_process::ScopedWaitableChild;

pub async fn collect(
    variable: &str,
    pattern: &str,
    mode: &str,
    instrumented: bool,
) -> Option<Value> {
    let binary = resources::binary(variable);
    let directory = tempfile::tempdir().unwrap();
    execute(&binary, directory.path(), pattern, mode).await;
    let raw = std::fs::read_dir(directory.path())
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension() == Some(OsStr::new(RAW_EXTENSION)))
        .collect::<Vec<_>>();
    if !instrumented {
        assert!(raw.is_empty());
        return None;
    }
    assert_eq!(raw.len(), SINGLE_PROFILE);
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
    Some(serde_json::from_slice(&commands::output(export).await).unwrap())
}

pub fn function<'a>(report: &'a Value, name: &str, expected: u64) -> &'a Value {
    let matches = report[DATA_FIELD]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|dataset| dataset[FUNCTIONS_FIELD].as_array().unwrap())
        .filter(|function| function[NAME_FIELD].as_str() == Some(name))
        .collect::<Vec<_>>();
    assert_eq!(matches.len(), SINGLE_FUNCTION, "{name}: {report}");
    let function = matches.into_iter().next().unwrap();
    assert_eq!(function[COUNT_FIELD].as_u64(), Some(expected), "{function}");
    function
}

async fn execute(binary: &Path, directory: &Path, pattern: &str, mode: &str) {
    let mut command = Command::new(binary);
    command
        .arg(mode)
        .env(PROFILE_ENV, directory.join(pattern))
        .env_remove(CONTINUOUS_ENV)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let deadline = tokio::time::Instant::now() + TIMEOUT;
    let mut child =
        ScopedWaitableChild::spawn_before_deadline(&mut command, deadline.into_std()).unwrap();
    drop(command);
    let mut stdout = tokio::process::ChildStdout::from_std(child.take_stdout().unwrap()).unwrap();
    let stderr = tokio::process::ChildStderr::from_std(child.take_stderr().unwrap()).unwrap();
    let execute = async {
        let mut ready = [0; READY.len()];
        stdout.read_exact(&mut ready).await.unwrap();
        assert_eq!(&ready, READY);
        if mode == BLOCKED {
            let status = child
                .terminate_and_wait_status(deadline.into_std())
                .await
                .unwrap();
            assert_eq!(status.signal(), Some(libc::SIGKILL));
        } else {
            child
                .wait_for_clean_exit(deadline.into_std())
                .await
                .unwrap();
        }
    };
    let (_, errors) = tokio::time::timeout_at(deadline, async {
        tokio::join!(execute, commands::read(stderr))
    })
    .await
    .unwrap();
    assert!(errors.is_empty(), "{}", String::from_utf8_lossy(&errors));
}
