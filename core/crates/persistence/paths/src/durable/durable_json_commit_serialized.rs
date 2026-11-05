pub(super) fn write_expected_entry_guarded_serialized<F, G, H>(
    entry: PrivateFileEntry,
    expected: Option<FileIdentity>,
    encoded: &[u8],
    authority: &ParentLockAuthority<'_>,
    guard: F,
    mut commit_fence: G,
    mut after_commit: H,
) -> Result<FileIdentity>
where
    F: FnOnce() -> Result<()>,
    G: FnMut() -> Result<()>,
    H: FnMut(FileIdentity),
{
    guard()?;
    write_expected_with_serialized_target(
        &entry,
        encoded,
        expected,
        authority,
        &mut |_| Ok(()),
        &mut || Ok(()),
        &mut commit_fence,
        &mut after_commit,
    )
}
