use yolo_hub_identity::RuntimeTarget;
use yolo_hub_terminal_protocol::{
    LeaseGrant, LeaseGrantFields, PublicError, SecretCapability, WriterPrincipal,
};
use yolo_terminal_model::{WriterLease, WriterLeaseGrant};

pub(crate) fn runtime(target: RuntimeTarget) -> yolo_runtime_protocol::RuntimeRef {
    yolo_runtime_protocol::RuntimeRef::new(
        target.address.terminal_id.as_uuid(),
        target.runtime_epoch.as_uuid(),
    )
}

pub(crate) fn public_grant(
    target: RuntimeTarget,
    principal: WriterPrincipal,
    grant: WriterLeaseGrant,
) -> Result<LeaseGrant, PublicError> {
    if grant.holder_id != principal.client_id.to_string()
        || grant.controller_instance_id.as_deref()
            != Some(principal.controller_instance_id.to_string().as_str())
    {
        return Err(PublicError::InvalidLease);
    }
    LeaseGrant::new(LeaseGrantFields {
        target,
        principal,
        generation: grant.generation,
        capability: SecretCapability::from_uuid(
            uuid::Uuid::from_slice(grant.writer_capability.as_bytes())
                .map_err(|_| PublicError::InvalidCapability)?,
        )?,
        expires_at_ms: grant.expires_at_ms,
        ttl_ms: grant.ttl_ms,
        renew_after_ms: grant.renew_after_ms,
    })
}

pub(crate) fn internal_grant(grant: LeaseGrant) -> Result<WriterLeaseGrant, PublicError> {
    let fields = grant.fields();
    let writer_capability = serde_json::from_value(
        serde_json::to_value(fields.capability).map_err(|_| PublicError::InvalidCapability)?,
    )
    .map_err(|_| PublicError::InvalidCapability)?;
    Ok(WriterLeaseGrant {
        lease: WriterLease {
            holder_id: fields.principal.client_id.to_string(),
            controller_instance_id: Some(fields.principal.controller_instance_id.to_string()),
            generation: fields.generation,
            expires_at_ms: fields.expires_at_ms,
            ttl_ms: fields.ttl_ms,
            renew_after_ms: fields.renew_after_ms,
        },
        writer_capability,
    })
}
