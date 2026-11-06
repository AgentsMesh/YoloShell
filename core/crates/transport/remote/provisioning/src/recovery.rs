use crate::{
    ProvisionGuard, RemoteProvisioningService, command, connection::FencedConnection, constants::*,
    journal, service::stage,
};
use anyhow::{Result, ensure};
use tokio::time::{Instant, timeout_at};
use yolo_distribution_bundle_source::VerifiedBundle;
use yolo_remote_provisioning_protocol::{EnrollmentOutcome, EnrollmentRecovery};
use yolo_remote::MultiplexedSshConnection;
use yolo_remote_bundle::RemoteBundleInstaller;
use yolo_remote_provisioning_protocol::{
    ProvisionOutcome, ProvisionPhase, ProvisionRequest, ProvisionStatus,
};

impl RemoteProvisioningService {
    /// Reads original outcomes only; never writes journal, uploads, publishes or bootstraps.
    pub async fn recover(
        &self,
        connection: &dyn MultiplexedSshConnection,
        guard: &dyn ProvisionGuard,
        request: &ProvisionRequest,
        deadline: Instant,
    ) -> Result<ProvisionStatus> {
        self.verify(request)?;
        guard.verify()?;
        let Some(observed) = journal::inspect(&self.parent, &self.root, request)? else {
            return ProvisionStatus::new(request.clone(), ProvisionOutcome::Absent {});
        };
        let record = &observed.record;
        let outcome = timeout_at(deadline, async {
            let source = VerifiedBundle::open(&request.fields().source)?;
            let check = || {
                guard.verify()?;
                self.verify(request)?;
                source.verify()?;
                observed.verify()?;
                ensure!(Instant::now() < deadline, DEADLINE);
                Ok(())
            };
            let connection = FencedConnection {
                inner: connection,
                guard: &check,
            };
            let stage = stage(request, &source)?;
            if record.phase == ProvisionPhase::Prepared {
                return Ok(ProvisionOutcome::Unknown {
                    phase: record.phase,
                });
            }
            let receipt = RemoteBundleInstaller
                .recover(&connection, &stage, &record.publication)
                .await?;
            if record.phase == ProvisionPhase::Publishing {
                return Ok(ProvisionOutcome::Unknown {
                    phase: record.phase,
                });
            }
            ensure!(Some(&receipt) == record.published.as_ref(), UNKNOWN);
            if record.phase == ProvisionPhase::Published {
                return Ok(ProvisionOutcome::Published {
                    publication: record.publication.clone(),
                    receipt,
                });
            }
            let bytes = command::run(&connection, request, &stage, true).await?;
            let recovered: EnrollmentRecovery = yolo_capnp_codec::decode(&bytes)?;
            recovered.validate_for(&request.fields().bootstrap)?;
            Ok::<_, anyhow::Error>(match recovered.fields().outcome.clone() {
                EnrollmentOutcome::Ready {
                    receipt: enrollment,
                } => ProvisionOutcome::Ready {
                    publication: record.publication.clone(),
                    receipt,
                    enrollment,
                },
                EnrollmentOutcome::Superseded {} => ProvisionOutcome::Superseded {
                    phase: record.phase,
                },
                _ => ProvisionOutcome::Unknown {
                    phase: record.phase,
                },
            })
        })
        .await
        .ok()
        .and_then(Result::ok)
        .unwrap_or(ProvisionOutcome::Unknown {
            phase: record.phase,
        });
        observed.verify()?;
        if guard.verify().is_err() {
            return ProvisionStatus::new(
                request.clone(),
                ProvisionOutcome::Unknown {
                    phase: record.phase,
                },
            );
        }
        ProvisionStatus::new(request.clone(), outcome)
    }
}
