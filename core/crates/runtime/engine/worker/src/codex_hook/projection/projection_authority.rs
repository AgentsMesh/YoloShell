use super::*;

pub(super) fn create(
    projected: &binding::ProjectedBinding,
    runtime: AuthorityRuntimeIdentity,
    boot_id: &str,
) -> Result<CodexBindingAuthority> {
    if projected.status == CodexAuthorityStatus::Active {
        CodexBindingAuthority::new_with_verified_resume_identity(
            runtime,
            projected.provider,
            projected.provider_version,
            projected.version,
            projected.status,
            projected.binding.clone(),
            boot_id,
        )
    } else {
        CodexBindingAuthority::new(
            runtime,
            projected.provider,
            projected.provider_version,
            projected.version,
            projected.status,
            projected.binding.clone(),
        )
    }
}
