use super::*;

pub(super) const SENTINEL_FILE_NAME: &str = "replacement-sentinel";
pub(super) const SENTINEL_BYTES: &[u8] = b"replacement-must-survive";
pub(super) const PRIMARY_STAGING_INDEX: usize = 0;

pub(super) fn replace_directory(source: &Path, moved: &Path, mode: u32) -> Result<()> {
    fs::rename(source, moved).context("move tested directory")?;
    fs::create_dir(source).context("create replacement directory")?;
    fs::write(source.join(SENTINEL_FILE_NAME), SENTINEL_BYTES)
        .context("write replacement sentinel")?;
    fs::set_permissions(source, fs::Permissions::from_mode(mode))
        .context("seal replacement directory")?;
    Ok(())
}

pub(super) fn pending_path(paths: &BrokerPaths) -> PathBuf {
    entries(paths)
        .find(|path| is_payload(path))
        .expect("pending generation is present")
}

pub(super) fn published_path(paths: &BrokerPaths) -> PathBuf {
    entries(paths)
        .find(|path| !name_starts_with(path, PENDING_GENERATION_PREFIX))
        .expect("published generation is present")
}

pub(super) fn first_pending(root: &Path) -> PathBuf {
    fs::read_dir(root)
        .unwrap()
        .filter_map(|entry| entry.ok().map(|value| value.path()))
        .find(|path| is_payload(path))
        .expect("staged generation is present")
}

fn entries(paths: &BrokerPaths) -> impl Iterator<Item = PathBuf> {
    fs::read_dir(paths.generations())
        .unwrap()
        .filter_map(|entry| entry.ok().map(|value| value.path()))
}

pub(in super::super) fn pending_count(paths: &BrokerPaths) -> usize {
    entries(paths)
        .filter(|path| name_starts_with(path, PENDING_GENERATION_PREFIX) && !is_payload(path))
        .count()
}

pub(in super::super) fn published_count(paths: &BrokerPaths) -> usize {
    entries(paths)
        .filter(|path| !name_starts_with(path, PENDING_GENERATION_PREFIX))
        .count()
}

fn name_starts_with(path: &Path, prefix: &str) -> bool {
    path.file_name()
        .is_some_and(|name| name.to_string_lossy().starts_with(prefix))
}

fn is_payload(path: &Path) -> bool {
    name_starts_with(path, PENDING_GENERATION_PREFIX)
        && path.file_name().is_some_and(|name| {
            name.to_string_lossy()
                .ends_with(PENDING_GENERATION_PAYLOAD_SUFFIX)
        })
}
