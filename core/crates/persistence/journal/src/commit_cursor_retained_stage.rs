use super::*;

pub(super) fn prepare_cursor_write(
    file: &File,
    identity: FileIdentity,
    path: &Path,
    current: CursorRecord,
    committed_bytes: u64,
) -> Result<(Vec<crate::paths::private_file::RetainedWrite>, CursorRecord), anyhow::Error> {
    let plan = super::slots::plan_retained_stage(file, identity, path, current, committed_bytes)?;
    let writes = plan
        .writes
        .into_iter()
        .map(|write| crate::paths::private_file::RetainedWrite {
            bytes: encode_cursor(write.record).to_vec(),
            offset: write.offset,
        })
        .collect();
    Ok((writes, plan.record))
}
