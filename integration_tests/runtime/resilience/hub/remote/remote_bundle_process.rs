use super::{constants::*, owners::Owners};
use std::{
    io::Read,
    path::Path,
    process::{Command, ExitStatus, Stdio},
};
use yolo_runtime::process::ScopedWaitableChild;

pub struct Invocation {
    pub status: ExitStatus,
    pub output: Vec<u8>,
    pub diagnostic: Vec<u8>,
    pub process: crate::ProcessIdentity,
}
impl Invocation {
    pub fn success(&self) {
        assert!(
            self.status.success(),
            "Hub CLI failed: {}",
            String::from_utf8_lossy(&self.diagnostic)
        );
        assert!(!self.process.matches());
    }
    pub fn failed(&self) {
        assert!(
            !self.status.success(),
            "remote bootstrap unexpectedly succeeded: {}",
            String::from_utf8_lossy(&self.output)
        );
        assert!(
            self.output.is_empty(),
            "failed bootstrap published a success document"
        );
        assert!(
            !self.process.matches(),
            "failed bootstrap left its caller alive: {:?}; stderr: {}",
            self.process,
            String::from_utf8_lossy(&self.diagnostic)
        );
    }
}

pub async fn invoke(command: &mut Command, root: &Path, owners: Option<&mut Owners>) -> Invocation {
    let output = tempfile::NamedTempFile::new_in(root).unwrap();
    let diagnostic = tempfile::NamedTempFile::new_in(root).unwrap();
    command
        .stdin(Stdio::null())
        .stdout(output.reopen().unwrap())
        .stderr(diagnostic.reopen().unwrap());
    let end = deadline();
    let mut child =
        ScopedWaitableChild::spawn_detached_before_deadline(command, end.into_std()).unwrap();
    let process = crate::ProcessIdentity::capture(child.process_id()).unwrap();
    let mut owners = owners;
    let status = loop {
        if let Some(owners) = owners.as_deref_mut() {
            owners.observe().await;
        }
        if let Some(status) = child.try_wait().unwrap() {
            child.retire_reaped().unwrap();
            break status;
        }
        if tokio::time::Instant::now() >= end {
            child
                .terminate_and_wait_reaped(deadline().into_std())
                .await
                .unwrap();
            crate::discard_incomplete_coverage_profile(process.pid);
            if let Some(owners) = owners.as_deref_mut() {
                owners.observe().await;
            }
            panic!(
                "Hub CLI exceeded acceptance deadline: {}",
                String::from_utf8_lossy(&capture(&diagnostic))
            );
        }
        tokio::time::sleep(POLL).await;
    };
    if let Some(owners) = owners.as_deref_mut() {
        loop {
            owners.observe().await;
            if !status.success() || owners.complete() || tokio::time::Instant::now() >= end {
                break;
            }
            tokio::time::sleep(POLL).await;
        }
    }
    Invocation {
        status,
        output: capture(&output),
        diagnostic: capture(&diagnostic),
        process,
    }
}

fn capture(file: &tempfile::NamedTempFile) -> Vec<u8> {
    let mut bytes = Vec::new();
    file.reopen()
        .unwrap()
        .take(MAX_CAPTURE_BYTES + CAPTURE_SENTINEL)
        .read_to_end(&mut bytes)
        .unwrap();
    assert!(bytes.len() as u64 <= MAX_CAPTURE_BYTES);
    bytes
}
