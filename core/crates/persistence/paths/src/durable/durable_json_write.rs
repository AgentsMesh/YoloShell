#[cfg(unix)]
fn write_unix_entry<F, G>(
    entry: &PrivateFileEntry,
    encoded: &[u8],
    hook: &mut F,
    commit: &mut G,
) -> Result<()>
where
    F: FnMut(DurableWriteStage) -> Result<()>,
    G: FnMut() -> Result<()>,
{
    let previous = entry.inspect(MAX_DURABLE_JSON_BYTES)?;
    write_expected(entry, encoded, previous, hook, commit)
}

pub(super) fn write_expected<F, G>(
    entry: &PrivateFileEntry,
    encoded: &[u8],
    previous: Option<FileIdentity>,
    hook: &mut F,
    commit: &mut G,
) -> Result<()>
where
    F: FnMut(DurableWriteStage) -> Result<()>,
    G: FnMut() -> Result<()>,
{
    let mut after_commit = |_| {};
    write_expected_with_fence(
        entry,
        encoded,
        previous,
        hook,
        commit,
        &mut || Ok(()),
        &mut after_commit,
    )
    .map(|_| ())
}

fn write_expected_with_fence<F, G, H, I>(
    entry: &PrivateFileEntry,
    encoded: &[u8],
    previous: Option<FileIdentity>,
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
    write_expected_with_temporary(
        entry,
        encoded,
        previous,
        hook,
        boundary,
        commit_fence,
        after_commit,
        PrivateFileEntry::temporary,
    )
}

pub(super) fn write_expected_with_target_lock_authority<F, G, H, I>(
    entry: &PrivateFileEntry,
    encoded: &[u8],
    previous: Option<FileIdentity>,
    authority: &TargetLockAuthority<'_>,
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
    authority.require_expected(previous)?;
    write_expected_with_temporary(
        entry,
        encoded,
        previous,
        hook,
        boundary,
        commit_fence,
        after_commit,
        |entry, prefix, suffix| {
            entry.temporary_with_target_lock_authority(prefix, suffix, authority)
        },
    )
}

fn write_expected_with_temporary<F, G, H, I, T>(
    entry: &PrivateFileEntry,
    encoded: &[u8],
    previous: Option<FileIdentity>,
    hook: &mut F,
    boundary: &mut G,
    commit_fence: &mut H,
    after_commit: &mut I,
    temporary_factory: T,
) -> Result<FileIdentity>
where
    F: FnMut(DurableWriteStage) -> Result<()>,
    G: FnMut() -> Result<()>,
    H: FnMut() -> Result<()>,
    I: FnMut(FileIdentity),
    T: Fn(&PrivateFileEntry, &str, &str) -> Result<TemporaryPrivateFile>,
{
    let mut temporary = temporary_factory(entry, ATOMIC_FILE_PREFIX, ATOMIC_FILE_SUFFIX)?;
    let result = write_and_install(
        entry,
        &mut temporary,
        previous,
        encoded,
        hook,
        boundary,
        commit_fence,
        after_commit,
    );
    if let Err(error) = result {
        if let Err(cleanup) = entry.remove_temporary(&temporary) {
            return Err(error.context(format!(
                "temporary durable state cleanup also failed: {cleanup:#}"
            )));
        }
        return Err(error);
    }
    result
}

#[cfg(unix)]
fn write_and_install<F, G, H, I>(
    entry: &PrivateFileEntry,
    temporary: &mut TemporaryPrivateFile,
    previous: Option<FileIdentity>,
    encoded: &[u8],
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
    temporary.file.write_all(encoded)?;
    temporary.sync_and_inspect(entry.path(), MAX_DURABLE_JSON_BYTES)?;
    hook(DurableWriteStage::TemporarySynced)?;
    entry.verify_expected(previous, MAX_DURABLE_JSON_BYTES)?;
    let committed_identity = temporary.identity;
    entry.rename_temporary_with_fences(
        temporary,
        MAX_DURABLE_JSON_BYTES,
        boundary,
        commit_fence,
        || after_commit(committed_identity),
    )?;
    hook(DurableWriteStage::Installed)?;
    entry.verify_current_content_version(temporary.identity, MAX_DURABLE_JSON_BYTES)?;
    let mut installed = entry.open_read(MAX_DURABLE_JSON_BYTES)?;
    let bytes = bounded_read(&mut installed.file, entry.path())?;
    installed.identity.verify_file_bounded(
        &installed.file,
        u64::try_from(bytes.len())?,
        entry.path(),
        MAX_DURABLE_JSON_BYTES,
    )?;
    anyhow::ensure!(bytes == encoded, "installed state content changed");
    entry.verify_current_unchanged(installed.identity, MAX_DURABLE_JSON_BYTES)?;
    Ok(installed.identity)
}
