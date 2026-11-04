use super::{records::ObservedRecord, *};
use anyhow::{Result, ensure};
use yolo_remote_broker::{
    BrokerRequest, BrokerRequestEnvelope, BrokerResponse, BrokerResult, BrokerSupervisionStore,
};
use yolo_runtime::{
    local_unix::LocalUnixStream,
    protocol::{read_frame, write_frame},
};

pub struct Custody {
    paths: BrokerPaths,
    registration: SshBrokerProfile,
    observed: Option<ObservedRecord>,
}
impl Custody {
    pub fn new(paths: &BrokerPaths, registration: &SshBrokerProfile) -> Self {
        Self {
            paths: paths.clone(),
            registration: registration.clone(),
            observed: None,
        }
    }
    pub async fn capture(&mut self) -> Result<BrokerStatus> {
        let (mut stream, pid) = self.connect().await?;
        let BrokerResult::Status(status) = request(&mut stream, BrokerRequest::Status).await?
        else {
            anyhow::bail!(INVALID_CUSTODY)
        };
        let observed = ObservedRecord::capture(&self.paths, &self.registration, &status, pid)?;
        let BrokerResult::Status(confirmed) = request(&mut stream, BrokerRequest::Status).await?
        else {
            anyhow::bail!(INVALID_CUSTODY)
        };
        ensure!(confirmed == status, INVALID_CUSTODY);
        observed.verify()?;
        self.paths.verify_state()?;
        if let Some(previous) = &self.observed {
            previous.verify()?;
            ensure!(previous.record == observed.record, INVALID_CUSTODY);
        }
        self.observed = Some(observed);
        Ok(status)
    }
    pub fn assert_live(&self) {
        let observed = self.observed.as_ref().unwrap();
        observed.verify().unwrap();
        assert!(observed.record.supervisor.process().matches());
        assert!(observed.record.broker.unwrap().process().matches());
    }
    pub async fn retire(&self) -> Result<()> {
        let observed = self
            .observed
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!(INVALID_CUSTODY))?;
        observed.verify()?;
        let (mut stream, pid) = self.connect().await?;
        ensure!(observed.record.broker.unwrap().pid == pid, INVALID_CUSTODY);
        let BrokerResult::Status(status) = request(&mut stream, BrokerRequest::Status).await?
        else {
            anyhow::bail!(INVALID_CUSTODY)
        };
        let current = ObservedRecord::capture(&self.paths, &self.registration, &status, pid)?;
        ensure!(current.record == observed.record, INVALID_CUSTODY);
        let result = request(
            &mut stream,
            BrokerRequest::Retire {
                expected_revision: status.config_revision.clone(),
                expected_instance_id: status.instance_id,
            },
        )
        .await?;
        ensure!(
            matches!(result, BrokerResult::Retire { instance_id, config_revision } if instance_id == status.instance_id && config_revision == status.config_revision),
            INVALID_CUSTODY
        );
        Ok(())
    }
    pub async fn wait(&mut self) {
        let observed = self.observed.as_ref().unwrap();
        tokio::time::timeout(CLEANUP_TIMEOUT, async {
            while !gone(observed) {
                tokio::time::sleep(POLL).await;
            }
        })
        .await
        .expect(CLEANUP_FAILURE);
        self.observed.take();
    }
    async fn connect(&self) -> Result<(LocalUnixStream, u32)> {
        use std::os::unix::fs::MetadataExt;
        self.paths.verify_state()?;
        let path = self.paths.management_socket();
        yolo_remote_broker::validate_broker_socket(&path)?;
        let before = std::fs::symlink_metadata(&path)?;
        let stream = tokio::time::timeout_at(deadline(), LocalUnixStream::connect(&path)).await??;
        let credentials = stream.peer_cred()?;
        ensure!(
            credentials.uid() == unsafe { libc::geteuid() },
            INVALID_CUSTODY
        );
        let pid = u32::try_from(
            credentials
                .pid()
                .ok_or_else(|| anyhow::anyhow!(INVALID_CUSTODY))?,
        )?;
        let after = std::fs::symlink_metadata(&path)?;
        ensure!(
            before.dev() == after.dev() && before.ino() == after.ino(),
            INVALID_CUSTODY
        );
        self.paths.verify_state()?;
        Ok((stream, pid))
    }
}

impl Drop for Custody {
    fn drop(&mut self) {
        let Some(observed) = &self.observed else {
            return;
        };
        if gone(observed) {
            return;
        }
        let requested = (|| -> Result<()> {
            self.paths.verify_state()?;
            observed.verify()?;
            let store = BrokerSupervisionStore::open_expected(
                self.paths.state_dir(),
                self.paths.state_anchor(),
            )?;
            observed.request_termination(&store)
        })();
        if let Err(error) = requested {
            tracing::error!(%error, "refusing unproven broker fixture cleanup");
            return;
        }
        let end = std::time::Instant::now() + CLEANUP_TIMEOUT;
        while !gone(observed) && std::time::Instant::now() < end {
            std::thread::sleep(POLL);
        }
        if !gone(observed) {
            tracing::error!("{CLEANUP_FAILURE}");
        }
    }
}

fn gone(observed: &ObservedRecord) -> bool {
    use yolo_process_test_support::ProcessIdentityObservation::Gone;
    observed.record.supervisor.process().observe() == Gone
        && observed.record.broker.unwrap().process().observe() == Gone
}
async fn request(stream: &mut LocalUnixStream, operation: BrokerRequest) -> Result<BrokerResult> {
    let request = BrokerRequestEnvelope::new(operation);
    tokio::time::timeout_at(deadline(), async {
        write_frame(stream, &request).await?;
        let response: BrokerResponse = read_frame(stream).await?;
        ensure!(
            response.protocol_version == yolo_remote_broker::BROKER_PROTOCOL_VERSION
                && response.request_id == request.request_id
                && response.error_code.is_none(),
            INVALID_CUSTODY
        );
        response
            .result
            .ok_or_else(|| anyhow::anyhow!(INVALID_CUSTODY))
    })
    .await?
}
