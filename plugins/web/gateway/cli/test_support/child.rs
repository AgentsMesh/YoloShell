use super::{configuration::Configuration, constants::*};
use std::{
    io::Read,
    net::SocketAddr,
    process::{Command, Stdio},
};
use tokio::time::{sleep, timeout_at};
use yolo_process_test_support::ScopedWaitableChild;

pub struct Child {
    child: Option<ScopedWaitableChild>,
    stdout: tempfile::NamedTempFile,
    stderr: tempfile::NamedTempFile,
}

impl Child {
    pub fn start(configuration: &Configuration) -> Self {
        let stdout = tempfile::NamedTempFile::new_in(configuration.root.path()).unwrap();
        let stderr = tempfile::NamedTempFile::new_in(configuration.root.path()).unwrap();
        let executable = std::fs::canonicalize(std::env::var_os(BINARY_ENV).unwrap()).unwrap();
        let mut command = Command::new(executable);
        command
            .args(configuration.arguments().into_iter().skip(1))
            .current_dir(configuration.root.path())
            .stdin(Stdio::null())
            .stdout(stdout.reopen().unwrap())
            .stderr(stderr.reopen().unwrap());
        let child = ScopedWaitableChild::spawn_before_deadline(&mut command, deadline().into_std())
            .unwrap();
        Self {
            child: Some(child),
            stdout,
            stderr,
        }
    }

    pub async fn ready(&mut self) -> SocketAddr {
        timeout_at(deadline(), async {
            loop {
                assert!(
                    self.child.as_mut().unwrap().try_wait().unwrap().is_none(),
                    "{}",
                    read(&self.stderr)
                );
                if let Some(address) = read(&self.stdout).lines().find_map(|line| {
                    line.strip_prefix(READY_PREFIX)
                        .and_then(|url| url.strip_suffix(yolo_web_gateway::WEB_PATH))
                }) {
                    return address.parse().unwrap();
                }
                sleep(POLL).await;
            }
        })
        .await
        .expect("CLI never published a listener")
    }

    pub async fn stop(mut self) {
        let child = self.child.take().unwrap();
        assert!(
            child
                .captured_identity()
                .unwrap()
                .signal(libc::SIGINT)
                .unwrap()
        );
        child
            .wait_for_clean_exit(deadline().into_std())
            .await
            .unwrap();
        assert!(read(&self.stderr).is_empty(), "{}", read(&self.stderr));
    }
}

fn read(file: &tempfile::NamedTempFile) -> String {
    let mut text = String::new();
    file.reopen()
        .unwrap()
        .take(MAX_LOG_BYTES)
        .read_to_string(&mut text)
        .unwrap();
    text
}
