use super::WorkerSecondaryCustodySource;
use super::secondary_custody_export::SuccessorCustodyExportLease;
use super::secondary_custody_source::InitialCustodyHandshakeBoundary;
use crate::pty_custody::{
    CustodyDescriptorSet, CustodyHolderIdentity, CustodyProfile, CustodyUnixStream,
};
use crate::sha256_digest::Sha256Digest;
use anyhow::Result;

pub(crate) enum WorkerCustodyHandshakeSource<'source> {
    Initial {
        source: &'source WorkerSecondaryCustodySource,
        profile: CustodyProfile,
    },
    Successor {
        export: &'source mut SuccessorCustodyExportLease,
        profile: CustodyProfile,
    },
    Released(Sha256Digest),
}

impl WorkerCustodyHandshakeSource<'_> {
    pub(crate) fn descriptor_set(&self) -> CustodyDescriptorSet {
        match self {
            Self::Initial { source, .. } => source.descriptor_set(),
            Self::Successor { export, .. } => export.source().descriptor_set(),
            Self::Released(_) => CustodyDescriptorSet::ReleasedSuccessor,
        }
    }

    pub(crate) fn profile(&self) -> CustodyProfile {
        match self {
            Self::Initial { profile, .. } => *profile,
            Self::Successor { profile, .. } => *profile,
            Self::Released(_) => CustodyProfile::ReleasedTerminal,
        }
    }

    pub(crate) fn recovery_identity_sha256(&self) -> Sha256Digest {
        match self {
            Self::Initial { source, .. } => source.recovery_identity_sha256(),
            Self::Successor { export, .. } => export.source().recovery_identity_sha256(),
            Self::Released(identity) => *identity,
        }
    }

    pub(crate) async fn send_descriptors(
        &mut self,
        stream: &CustodyUnixStream,
        holder: CustodyHolderIdentity,
    ) -> Result<()> {
        match self {
            Self::Initial { source, .. } => source.send_descriptors_unfenced(stream, holder).await,
            Self::Successor { export, .. } => export.send_descriptors(stream, holder).await,
            Self::Released(_) => stream.send_rights(&[]).await,
        }
    }

    pub(crate) fn inject_initial_fault(
        &self,
        boundary: InitialCustodyHandshakeBoundary,
    ) -> Result<()> {
        match self {
            Self::Initial { source, .. } => source.inject_initial_fault(boundary),
            Self::Successor { export, .. } => export.source().inject_initial_fault(boundary),
            Self::Released(_) => Ok(()),
        }
    }
}
