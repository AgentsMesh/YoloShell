use crate::{
    ProvisionGuard, RemoteProvisioningService, command, connection::FencedConnection, constants::*,
    journal, owner::Owner,
};
use anyhow::{Result, ensure};
use tokio::time::{Instant, timeout_at};
use yolo_distribution_bundle::ArtifactRole;
use yolo_distribution_bundle_source::VerifiedBundle;
use yolo_remote_provisioning_protocol::RemoteBootstrapReceipt;
use yolo_remote::{BundlePayloads, BundleStageRequest, MultiplexedSshConnection};
use yolo_remote_bundle::RemoteBundleInstaller;
use yolo_remote_provisioning_protocol::{
    ProvisionOutcome, ProvisionPhase, ProvisionRequest, ProvisionStatus,
};

impl RemoteProvisioningService {
    pub async fn provision(
        &self,
        connection: &dyn MultiplexedSshConnection,
        guard: &dyn ProvisionGuard,
        request: &ProvisionRequest,
        deadline: Instant,
    ) -> Result<ProvisionStatus> {
        self.verify(request)?;
        guard.verify()?;
        let lock = self.lock(request)?;
        if journal::inspect(&self.parent, &self.root, request)?.is_some() {
            drop(lock);
            return self.recover(connection, guard, request, deadline).await;
        }
        let source = VerifiedBundle::open(&request.fields().source)?;
        let check = || {
            guard.verify()?;
            self.verify(request)?;
            source.verify()?;
            ensure!(Instant::now() < deadline, DEADLINE);
            Ok(())
        };
        let connection = FencedConnection {
            inner: connection,
            guard: &check,
        };
        let stage = stage(request, &source)?;
        let publication = timeout_at(
            deadline,
            RemoteBundleInstaller.prepare(&connection, &stage, request.fields().request_id),
        )
        .await??;
        let mut owner = self.prepare(request, publication, lock, &check)?;
        let result = timeout_at(deadline, async {
            owner.update(&check, |record| record.phase = ProvisionPhase::Publishing)?;
            let published = RemoteBundleInstaller
                .publish(&connection, &stage, &owner.observed.record.publication)
                .await?;
            owner.update(&check, |record| {
                record.phase = ProvisionPhase::Published;
                record.published = Some(published);
            })?;
            enroll(&connection, &check, &stage, &mut owner).await
        })
        .await;
        settled(result.ok().and_then(Result::ok), &owner, &check)
    }

    /// Explicitly starts only the never-attempted enrollment edge of a known Published record.
    pub async fn continue_known_stage(
        &self,
        connection: &dyn MultiplexedSshConnection,
        guard: &dyn ProvisionGuard,
        request: &ProvisionRequest,
        deadline: Instant,
    ) -> Result<ProvisionStatus> {
        self.verify(request)?;
        guard.verify()?;
        let lock = self.lock(request)?;
        let mut owner = self.advance_owner(request, lock)?;
        let source = VerifiedBundle::open(&request.fields().source)?;
        let check = || {
            guard.verify()?;
            self.verify(request)?;
            source.verify()?;
            ensure!(Instant::now() < deadline, DEADLINE);
            Ok(())
        };
        let connection = FencedConnection {
            inner: connection,
            guard: &check,
        };
        let stage = stage(request, &source)?;
        let result = timeout_at(deadline, async {
            owner.verify(&check)?;
            let published = RemoteBundleInstaller
                .recover(&connection, &stage, &owner.observed.record.publication)
                .await?;
            ensure!(
                Some(&published) == owner.observed.record.published.as_ref(),
                UNKNOWN
            );
            enroll(&connection, &check, &stage, &mut owner).await
        })
        .await;
        settled(result.ok().and_then(Result::ok), &owner, &check)
    }
}
async fn enroll(
    connection: &dyn MultiplexedSshConnection,
    guard: &dyn ProvisionGuard,
    stage: &BundleStageRequest,
    owner: &mut Owner,
) -> Result<()> {
    owner.update(guard, |record| record.phase = ProvisionPhase::Enrolling)?;
    let bytes = command::run(connection, &owner.observed.record.request, stage, false).await?;
    let receipt: RemoteBootstrapReceipt = yolo_capnp_codec::decode(&bytes)?;
    receipt.validate_for(&owner.observed.record.request.fields().bootstrap)?;
    owner.update(guard, |record| {
        record.phase = ProvisionPhase::Complete;
        record.enrollment = Some(receipt);
    })
}
fn settled(
    success: Option<()>,
    owner: &Owner,
    guard: &dyn ProvisionGuard,
) -> Result<ProvisionStatus> {
    owner.observed.verify()?;
    let record = &owner.observed.record;
    let outcome = if success.is_some() && guard.verify().is_ok() {
        ProvisionOutcome::Ready {
            publication: record.publication.clone(),
            receipt: record
                .published
                .clone()
                .ok_or_else(|| anyhow::anyhow!(INVALID))?,
            enrollment: record
                .enrollment
                .clone()
                .ok_or_else(|| anyhow::anyhow!(INVALID))?,
        }
    } else {
        ProvisionOutcome::Unknown {
            phase: record.phase,
        }
    };
    ProvisionStatus::new(record.request.clone(), outcome)
}
pub(crate) fn stage(
    request: &ProvisionRequest,
    source: &VerifiedBundle,
) -> Result<BundleStageRequest> {
    let fields = request.fields();
    Ok(BundleStageRequest::new(
        fields.install_root.clone(),
        fields.request_id,
        fields.source.descriptor().clone(),
        BundlePayloads {
            runtime: source.payload(ArtifactRole::Runtime)?,
        },
    )?)
}
