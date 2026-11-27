use super::exchange_guard::PostPreparedFailStop;
use super::*;
use crate::pty_custody::{CustodyHostAck, CustodyRequest, CustodyWorkerCommit};

const CUSTODY_TIMEOUT_DETAIL: &str = "PTY secondary-custody handshake timed out";
const CONNECT_CONTEXT: &str = "connect to PTY secondary-custody worker";
const OPENING_WRITE_CONTEXT: &str = "send PTY custody opening request";
const OFFER_READ_CONTEXT: &str = "read PTY custody descriptor offer";
const RIGHTS_READ_CONTEXT: &str = "receive PTY custody descriptors";
const PREPARED_WRITE_CONTEXT: &str = "send PTY custody Prepared acknowledgement";
const COMMIT_READ_CONTEXT: &str = "read PTY custody commit";
const COMMIT_MISMATCH_DETAIL: &str = "PTY custody commit does not match its exact offer";
const PREPARED_STREAM_MISSING_DETAIL: &str = "PTY custody Prepared stream is unavailable";
const HANDSHAKE_PANIC_CONTEXT: &str = "PTY secondary-custody handshake panicked";
const INCOMPLETE_LEASE_DETAIL: &str =
    "same-generation PTY custody activation cannot resume an incomplete lease";
const RETIRED_LEASE_DETAIL: &str =
    "exact PTY custody runtime has already retired from this Host generation";
const RETIRING_LEASE_DETAIL: &str =
    "exact PTY custody runtime is awaiting its terminal release acknowledgement";
const UNCERTAIN_RELEASE_DETAIL: &str =
    "exact PTY custody runtime has an uncertain terminal release";
const TRANSITIONING_LEASE_DETAIL: &str =
    "exact PTY custody runtime is converging after a monitor fault";

impl Host {
    pub(in crate::host) async fn ensure_secondary_custody(
        &self,
        record: &RuntimeRecord,
    ) -> Result<(), WorkerRpcError> {
        if let Some(detail) = self.request_drain.fail_stop_detail() {
            return Err(WorkerRpcError::transport(detail));
        }
        let mut fail_stop = PostPreparedFailStop::new(self.request_drain.clone());
        let operation = super::containment::catch_future_detail(
            self.ensure_secondary_custody_inner(record, &mut fail_stop),
        );
        let result = match tokio::time::timeout(self.worker_ready_timeout, operation).await {
            Ok(Ok(result)) => result,
            Ok(Err(detail)) => Err(WorkerRpcError::protocol(format!(
                "{HANDSHAKE_PANIC_CONTEXT}: {detail}"
            ))),
            Err(_) => Err(WorkerRpcError::transport(CUSTODY_TIMEOUT_DETAIL)),
        };
        if let Err(error) = &result {
            fail_stop.record_failure(error);
        }
        result
    }

    async fn ensure_secondary_custody_inner(
        &self,
        record: &RuntimeRecord,
        fail_stop: &mut PostPreparedFailStop,
    ) -> Result<(), WorkerRpcError> {
        let runtime = CustodyRuntimeIdentity::from_record(record);
        validation::validate_runtime(self, record, &runtime)?;
        fail_stop.bind_gate(self.secondary_custody.lock(record.terminal_id).await);
        if let Some(detail) = self.request_drain.fail_stop_detail() {
            return Err(WorkerRpcError::transport(detail));
        }
        match self.secondary_custody.admission(&runtime) {
            SecondaryCustodyAdmission::Retired {
                runtime: retired,
                state: SecondaryCustodyState::ReleaseAckPending,
            } if retired == runtime => {
                return Err(WorkerRpcError::transport(RETIRING_LEASE_DETAIL));
            }
            SecondaryCustodyAdmission::Retired {
                runtime: retired,
                state: SecondaryCustodyState::ReleaseUncertain,
            } if retired == runtime => {
                return Err(WorkerRpcError::protocol(UNCERTAIN_RELEASE_DETAIL));
            }
            SecondaryCustodyAdmission::Retired { .. } => {
                return Err(WorkerRpcError::protocol(RETIRED_LEASE_DETAIL));
            }
            SecondaryCustodyAdmission::Active {
                lease,
                state,
                ready,
            } => {
                validation::validate_lease(self, record, &lease)?;
                if ready {
                    return Ok(());
                }
                if matches!(
                    state,
                    SecondaryCustodyState::Disconnected | SecondaryCustodyState::Failed
                ) {
                    return Err(WorkerRpcError::transport(TRANSITIONING_LEASE_DETAIL));
                }
                fail_stop.arm();
                return Err(WorkerRpcError::protocol(INCOMPLETE_LEASE_DETAIL));
            }
            SecondaryCustodyAdmission::Absent => {}
        }
        let successor = self
            .exchange_secondary_custody(record, runtime, fail_stop)
            .await?;
        self.activate_secondary_custody(successor, fail_stop).await
    }

    async fn exchange_secondary_custody(
        &self,
        record: &RuntimeRecord,
        runtime: CustodyRuntimeIdentity,
        fail_stop: &mut PostPreparedFailStop,
    ) -> Result<Arc<SecondaryCustodyLease>, WorkerRpcError> {
        let predecessor = validation::validate_durable_candidate(self, record, &runtime)?;
        let request = CustodyRequest::new(
            crate::pty_custody::CustodyLeaseId::new(),
            runtime,
            self.secondary_custody_holder,
            record.auth_token.clone(),
            self.controller_generation,
        );
        let mut stream = crate::pty_custody::CustodyUnixStream::connect(&record.socket_path)
            .await
            .map_err(|error| transport(CONNECT_CONTEXT, error))?;
        peer_validation::validate(self, &stream, record)?;
        fail_stop.arm();
        stream
            .write_opening(&request)
            .await
            .map_err(|error| transport(OPENING_WRITE_CONTEXT, error))?;
        let offer = stream
            .read()
            .await
            .map_err(|error| transport(OFFER_READ_CONTEXT, error))?;
        validation::validate_offer(&request, &offer, predecessor.record())?;
        let descriptors = stream
            .receive_rights(offer.descriptor_set.descriptor_count())
            .await
            .map_err(|error| transport(RIGHTS_READ_CONTEXT, error))?;
        let platform = self.import_secondary_custody(record, &offer, descriptors)?;
        let guardian = self
            .prepare_secondary_custody_guardian(record, &request, &offer, &predecessor, &platform)
            .await?;
        let lease = Arc::new(SecondaryCustodyLease::new(
            self, &request, offer, guardian, platform, stream,
        )?);
        self.secondary_custody.install(Arc::clone(&lease))?;
        self.complete_secondary_custody_commit(record, &request, &lease)
            .await?;
        Ok(lease)
    }

    async fn complete_secondary_custody_commit(
        &self,
        record: &RuntimeRecord,
        request: &CustodyRequest,
        lease: &SecondaryCustodyLease,
    ) -> Result<(), WorkerRpcError> {
        let mut slot = lease.stream.lock().await;
        let stream = slot
            .as_mut()
            .ok_or_else(|| WorkerRpcError::protocol(PREPARED_STREAM_MISSING_DETAIL))?;
        stream
            .write(&match lease.guardian {
                Some(guardian) => CustodyHostAck::prepared_guarded(&lease.offer, guardian),
                None => CustodyHostAck::prepared(&lease.offer),
            })
            .await
            .map_err(|error| transport(PREPARED_WRITE_CONTEXT, error))?;
        let commit: CustodyWorkerCommit = stream
            .read()
            .await
            .map_err(|error| transport(COMMIT_READ_CONTEXT, error))?;
        if !commit.matches_offer(&lease.offer) {
            return Err(WorkerRpcError::identity_mismatch(COMMIT_MISMATCH_DETAIL));
        }
        validation::validate_committed(
            self,
            record,
            request,
            &lease.offer,
            lease.guardian.map(|preparation| preparation.binding()),
        )
    }
}

fn transport(context: &'static str, error: anyhow::Error) -> WorkerRpcError {
    let detail = super::containment::error_detail(error);
    WorkerRpcError::transport(format!("{context}: {detail}"))
}
