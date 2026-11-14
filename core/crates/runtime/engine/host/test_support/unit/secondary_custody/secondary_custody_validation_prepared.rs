use super::*;
use crate::pty_custody::{
    CustodyDescriptorSet, CustodyDurableLease, CustodyDurableState, CustodyGuardianBinding,
    CustodyHolderIdentity, CustodyLeaseId, CustodyRequest, CustodyRuntimeIdentity,
};
use crate::sha256_digest::Sha256Digest;

const AUTH_TOKEN: &str = "prepared-validation-token";
const FIRST_GENERATION: u64 = 1;
const NEXT_GENERATION: u64 = 2;
const OFFER_GENERATION: u64 = 3;
const READER_HANDLE: u64 = 11;
const WRITER_HANDLE: u64 = 12;
pub(super) const FOREIGN_READER_HANDLE: u64 = 21;
pub(super) const FOREIGN_WRITER_HANDLE: u64 = 22;
const DIGEST_BYTE: u8 = 0x35;
pub(super) const FOREIGN_DIGEST_BYTE: u8 = 0x53;

pub(super) struct Fixture {
    pub(super) durable: CustodyDurableRecord,
    pub(super) offer: CustodyOffer,
    pub(super) gone: CustodyHolderIdentity,
    pub(super) current: CustodyHolderIdentity,
}

impl Fixture {
    pub(super) fn new(
        previous_profile: CustodyProfile,
        candidate_profile: CustodyProfile,
        offer_profile: CustodyProfile,
    ) -> Self {
        let current = CustodyHolderIdentity::capture().unwrap();
        let reaped = crate::host::reaped_host_test_process_identity();
        let gone = CustodyHolderIdentity {
            pid: reaped.pid,
            start_time: reaped.start_time,
        };
        assert_eq!(gone.observation(), CustodyHolderObservation::ProvenGone);
        let runtime = CustodyRuntimeIdentity {
            terminal_id: uuid::Uuid::new_v4(),
            runtime_epoch: uuid::Uuid::new_v4(),
            worker_id: uuid::Uuid::new_v4(),
            worker_pid: current.pid,
            worker_start_time: current.start_time,
            child_pid: current.pid,
            child_start_time: current.start_time,
            boot_id: crate::process::boot_id(),
        };
        let previous_request = request(&runtime, gone, FIRST_GENERATION);
        let previous_offer = offer(&previous_request, previous_profile);
        let guardian_id = uuid::Uuid::new_v4();
        let guardian =
            CustodyGuardianBinding::new(guardian_id, current, &previous_request, &previous_offer)
                .unwrap();
        let previous = CustodyDurableRecord::pending(runtime.clone())
            .prepare_guardian(&previous_request, &previous_offer, guardian)
            .unwrap()
            .arm_prepared_guardian(&previous_request, &previous_offer, guardian)
            .unwrap();
        let candidate_request = request(&runtime, gone, NEXT_GENERATION);
        let candidate_offer = offer(&candidate_request, candidate_profile);
        let candidate_guardian =
            CustodyGuardianBinding::new(guardian_id, current, &candidate_request, &candidate_offer)
                .unwrap();
        let durable = previous
            .prepare_guardian(&candidate_request, &candidate_offer, candidate_guardian)
            .unwrap();
        assert!(durable.is_valid());
        let offer = offer(&request(&runtime, current, OFFER_GENERATION), offer_profile);
        Self {
            durable,
            offer,
            gone,
            current,
        }
    }

    pub(super) fn active() -> Self {
        Self::new(
            CustodyProfile::ActiveRunning,
            CustodyProfile::ActiveRunning,
            CustodyProfile::ActiveRunning,
        )
    }

    pub(super) fn candidate(&mut self) -> &mut CustodyDurableLease {
        let CustodyDurableState::PreparedGuardian { candidate, .. } = &mut self.durable.state
        else {
            unreachable!()
        };
        candidate
    }

    pub(super) fn replace_guardian(&mut self, process: CustodyHolderIdentity) {
        let candidate = *self.candidate();
        let mut request = request(
            &self.durable.runtime,
            candidate.holder,
            candidate.controller_generation,
        );
        request.lease_id = candidate.lease_id;
        let mut offer = offer(&request, candidate.profile);
        offer.offer_id = candidate.offer_id;
        offer.descriptor_set = candidate.descriptor_set;
        offer.recovery_identity_sha256 = candidate.recovery_identity_sha256;
        self.candidate().guardian = CustodyGuardianBinding::new(
            candidate.guardian.unwrap().guardian_id(),
            process,
            &request,
            &offer,
        );
        assert!(self.durable.is_valid());
    }

    pub(super) fn replace_holder(&mut self, holder: CustodyHolderIdentity) {
        let guardian = self.candidate().guardian.unwrap().process();
        self.candidate().holder = holder;
        self.replace_guardian(guardian);
    }
}

fn request(
    runtime: &CustodyRuntimeIdentity,
    holder: CustodyHolderIdentity,
    generation: u64,
) -> CustodyRequest {
    CustodyRequest::new(
        CustodyLeaseId::new(),
        runtime.clone(),
        holder,
        AUTH_TOKEN.to_owned(),
        generation,
    )
}

fn offer(request: &CustodyRequest, profile: CustodyProfile) -> CustodyOffer {
    CustodyOffer::new_with_profile(
        request,
        CustodyDescriptorSet::macos_process_capability(READER_HANDLE, WRITER_HANDLE),
        profile,
        Sha256Digest::repeated(DIGEST_BYTE),
    )
    .unwrap()
}
