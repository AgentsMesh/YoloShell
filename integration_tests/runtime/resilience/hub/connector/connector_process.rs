use super::{super::federation_support::deadline, constants::*};
use std::{
    os::{
        fd::OwnedFd,
        unix::{net::UnixStream, process::ExitStatusExt},
    },
    path::Path,
    process::{Command as ProcessCommand, Stdio},
};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use yolo_hub_connector_protocol::{Command, Reply, Request, Response};
use yolo_hub_identity::HubId;
use yolo_runtime::process::ScopedWaitableChild;

pub struct Publisher {
    child: ScopedWaitableChild,
    input: tokio::net::UnixStream,
    output: BufReader<tokio::process::ChildStdout>,
    hub: HubId,
}
impl Publisher {
    pub async fn start(socket: &Path, hub: HubId) -> Self {
        let executable =
            crate::bazel_runfile(&std::env::var(CONNECTOR_BINARY_ENV).unwrap()).unwrap();
        let (input, child_input) = UnixStream::pair().unwrap();
        let child_input: OwnedFd = child_input.into();
        let mut command = ProcessCommand::new(executable);
        command
            .arg(SOCKET_ARGUMENT)
            .arg(socket)
            .arg(HUB_ARGUMENT)
            .arg(hub.to_string())
            .stdin(Stdio::from(child_input))
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit());
        let mut child = ScopedWaitableChild::spawn_detached_before_deadline(
            &mut command,
            deadline().into_std(),
        )
        .unwrap();
        input.set_nonblocking(true).unwrap();
        let output = tokio::process::ChildStdout::from_std(child.take_stdout().unwrap()).unwrap();
        let mut publisher = Self {
            child,
            input: tokio::net::UnixStream::from_std(input).unwrap(),
            output: BufReader::new(output),
            hub,
        };
        assert!(
            matches!(publisher.call(Command::Hello {}).await, Reply::Hello { hub: actual } if actual == hub)
        );
        publisher
    }
    pub async fn call(&mut self, command: Command) -> Reply {
        let request = Request::new(self.hub, command).unwrap();
        let mut bytes = serde_json::to_vec(&request).unwrap();
        bytes.extend_from_slice(LINE_TERMINATOR);
        tokio::time::timeout_at(deadline(), self.input.write_all(&bytes))
            .await
            .unwrap()
            .unwrap();
        let mut line = Vec::new();
        tokio::time::timeout_at(deadline(), self.output.read_until(LINE_END, &mut line))
            .await
            .unwrap()
            .unwrap();
        assert!(!line.is_empty() && line.len() <= MAX_RESPONSE_BYTES);
        let response: Response = serde_json::from_slice(&line).unwrap();
        response.validate_for(&request).unwrap();
        response.reply
    }
    pub fn pid(&self) -> u32 {
        self.child.process_id()
    }
    pub async fn stop(self) {
        drop(self.input);
        self.child
            .wait_for_clean_exit(deadline().into_std())
            .await
            .unwrap();
    }
    pub async fn crash(self) {
        let pid = self.child.process_id();
        let status = self
            .child
            .terminate_and_wait_status(deadline().into_std())
            .await
            .unwrap();
        assert_eq!(status.signal(), Some(libc::SIGKILL));
        crate::discard_incomplete_coverage_profile(pid);
    }
}
