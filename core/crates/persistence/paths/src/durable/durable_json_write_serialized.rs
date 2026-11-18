pub(super) fn write_expected_with_serialized_target<F, G, H, I>(
    entry: &PrivateFileEntry,
    encoded: &[u8],
    previous: Option<FileIdentity>,
    authority: &ParentLockAuthority<'_>,
    hook: &mut F,
    boundary: &mut G,
    commit_fence: &mut H,
    after_commit: &mut I,
) -> Result<FileIdentity>
where
    F: FnMut(DurableWriteStage) -> Result<()>,
    G: FnMut() -> Result<()>,
    H: FnMut() -> Result<()>,
    I: FnMut(FileIdentity),
{
    authority.require_entry(entry)?;
    let mut serialized_commit = || {
        commit_fence()?;
        authority.require_entry(entry)
    };
    let identity = write_expected_with_temporary(
        entry,
        encoded,
        previous,
        hook,
        boundary,
        &mut serialized_commit,
        after_commit,
        |entry, prefix, suffix| entry.temporary_with_serialized_target(prefix, suffix, authority),
    )?;
    authority.require_entry(entry)?;
    Ok(identity)
}
