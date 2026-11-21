use super::super::pty_master_release::PtyMasterAliasVerifier;

pub(super) enum PtyMasterAliasState {
    Unbound,
    AliasBound(PtyMasterAliasVerifier),
    NoLocalAlias,
}
