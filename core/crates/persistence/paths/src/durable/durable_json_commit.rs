#[cfg(unix)]
pub(super) fn write_entry_guarded<F>(
    entry: PrivateFileEntry,
    encoded: &[u8],
    guard: F,
) -> Result<()>
where
    F: FnOnce() -> Result<()>,
{
    guard()?;
    write_unix_entry(&entry, encoded, &mut |_| Ok(()), &mut || Ok(()))
}

fn write_unix_entry_with_fence<F, G, H>(
    entry: &PrivateFileEntry,
    encoded: &[u8],
    hook: &mut F,
    boundary: &mut G,
    commit_fence: &mut H,
) -> Result<()>
where
    F: FnMut(DurableWriteStage) -> Result<()>,
    G: FnMut() -> Result<()>,
    H: FnMut() -> Result<()>,
{
    let previous = entry.inspect(MAX_DURABLE_JSON_BYTES)?;
    write_expected_with_fence(
        entry,
        encoded,
        previous,
        hook,
        boundary,
        commit_fence,
        &mut |_| {},
    )
    .map(|_| ())
}

pub(super) fn write_entry_guarded_with_commit_fence<F, G>(
    entry: PrivateFileEntry,
    encoded: &[u8],
    guard: F,
    mut commit_fence: G,
) -> Result<()>
where
    F: FnOnce() -> Result<()>,
    G: FnMut() -> Result<()>,
{
    guard()?;
    write_unix_entry_with_fence(
        &entry,
        encoded,
        &mut |_| Ok(()),
        &mut || Ok(()),
        &mut commit_fence,
    )
}

pub(super) fn write_expected_entry_guarded_with_commit_fence<F, G>(
    entry: PrivateFileEntry,
    expected: Option<FileIdentity>,
    encoded: &[u8],
    guard: F,
    mut commit_fence: G,
) -> Result<FileIdentity>
where
    F: FnOnce() -> Result<()>,
    G: FnMut() -> Result<()>,
{
    guard()?;
    write_expected_with_fence(
        &entry,
        encoded,
        expected,
        &mut |_| Ok(()),
        &mut || Ok(()),
        &mut commit_fence,
        &mut |_| {},
    )
}

pub fn write_expected_entry_guarded_with_commit_observer<F, G, H>(
    entry: PrivateFileEntry,
    expected: Option<FileIdentity>,
    encoded: &[u8],
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
    write_expected_with_fence(
        &entry,
        encoded,
        expected,
        &mut |_| Ok(()),
        &mut || Ok(()),
        &mut commit_fence,
        &mut after_commit,
    )
}

pub(super) fn write_expected_entry_guarded_with_commit_fence_with_target_lock_authority<F, G>(
    entry: PrivateFileEntry,
    expected: Option<FileIdentity>,
    encoded: &[u8],
    authority: &TargetLockAuthority<'_>,
    guard: F,
    mut commit_fence: G,
) -> Result<FileIdentity>
where
    F: FnOnce() -> Result<()>,
    G: FnMut() -> Result<()>,
{
    guard()?;
    write_expected_with_target_lock_authority(
        &entry,
        encoded,
        expected,
        authority,
        &mut |_| Ok(()),
        &mut || Ok(()),
        &mut commit_fence,
        &mut |_| {},
    )
}

pub(super) fn write_expected_entry_guarded_with_commit_observer_with_target_lock_authority<
    F,
    G,
    H,
>(
    entry: PrivateFileEntry,
    expected: Option<FileIdentity>,
    encoded: &[u8],
    authority: &TargetLockAuthority<'_>,
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
    write_expected_with_target_lock_authority(
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

pub(super) fn replace_expected_entry<F>(
    entry: PrivateFileEntry,
    expected: FileIdentity,
    encoded: &[u8],
    guard: F,
) -> Result<()>
where
    F: FnOnce() -> Result<()>,
{
    guard()?;
    write_expected(
        &entry,
        encoded,
        Some(expected),
        &mut |_| Ok(()),
        &mut || Ok(()),
    )
}
