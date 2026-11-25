use crate::identity::{Generation, HookIdentity, ProcessIdentity, HookProviderKind, ProviderVersion};
use crate::snapshot::wire::{BindingDto, IdentityDto, ProcessIdentityDto, ProviderDto, VersionDto};
use crate::state::NormalizedLifecycle;

pub(crate) fn binding_to_dto(state: &NormalizedLifecycle) -> BindingDto {
    BindingDto {
        identity: identity_to_dto(&state.expected_identity),
        provider: provider_to_dto(&state.provider),
        provider_version: version_to_dto(state.provider_version),
    }
}

pub(crate) fn identity_to_dto(identity: &HookIdentity) -> IdentityDto {
    IdentityDto {
        terminal_id: identity.terminal_id,
        runtime_epoch: identity.runtime_epoch,
        invocation_id: identity.invocation_id,
        hook_token: identity.hook_token,
        process: ProcessIdentityDto {
            pid: identity.process.pid,
            start_time: identity.process.start_time,
        },
        generation: identity.generation.0,
    }
}

pub(crate) fn identity_from_dto(identity: IdentityDto) -> HookIdentity {
    HookIdentity {
        terminal_id: identity.terminal_id,
        runtime_epoch: identity.runtime_epoch,
        invocation_id: identity.invocation_id,
        hook_token: identity.hook_token,
        process: ProcessIdentity::new(identity.process.pid, identity.process.start_time),
        generation: Generation(identity.generation),
    }
}

pub(crate) fn provider_to_dto(provider: &HookProviderKind) -> ProviderDto {
    match provider {
        HookProviderKind::Codex => ProviderDto::Codex {},
        HookProviderKind::Claude => ProviderDto::Claude {},
        HookProviderKind::Other(name) => ProviderDto::Other { name: name.clone() },
    }
}

pub(crate) fn provider_from_dto(provider: ProviderDto) -> HookProviderKind {
    match provider {
        ProviderDto::Codex {} => HookProviderKind::Codex,
        ProviderDto::Claude {} => HookProviderKind::Claude,
        ProviderDto::Other { name } => HookProviderKind::Other(name),
    }
}

pub(crate) const fn version_to_dto(version: ProviderVersion) -> VersionDto {
    VersionDto {
        major: version.major,
        minor: version.minor,
        patch: version.patch,
    }
}

pub(crate) const fn version_from_dto(version: VersionDto) -> ProviderVersion {
    ProviderVersion::new(version.major, version.minor, version.patch)
}
