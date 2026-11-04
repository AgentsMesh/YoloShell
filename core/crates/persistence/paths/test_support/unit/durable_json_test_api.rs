pub(super) fn write_with_hook<T, F>(path: &Path, value: &T, stage_hook: F) -> Result<()>
where
    T: Serialize,
    F: FnMut(DurableWriteStage) -> Result<()>,
{
    write_unix(path, &encode(path, value)?, stage_hook)
}

pub(super) fn write_with_boundary_hook<T, F>(path: &Path, value: &T, hook: F) -> Result<()>
where
    T: Serialize,
    F: FnMut() -> Result<()>,
{
    write_unix_with(path, &encode(path, value)?, |_| Ok(()), hook)
}

pub(super) fn read_with_hook<T, F>(path: &Path, hook: F) -> Result<T>
where
    T: DeserializeOwned,
    F: FnMut(DurableReadStage) -> Result<()>,
{
    read_unix(path, hook)
}
