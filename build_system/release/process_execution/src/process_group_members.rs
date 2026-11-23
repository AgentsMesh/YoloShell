use crate::constants::{MAX_PROCESS_GROUP_MEMBERS, PROCESS_GROUP_ONLY_FILTER};
use anyhow::{Context, Result, ensure};

pub(crate) fn has_other_members(group: libc::pid_t, leader: libc::pid_t) -> Result<bool> {
    let mut processes = vec![0 as libc::pid_t; MAX_PROCESS_GROUP_MEMBERS];
    let byte_capacity = processes
        .len()
        .checked_mul(std::mem::size_of::<libc::pid_t>())
        .context("process group buffer size overflow")?;
    let byte_capacity = libc::c_int::try_from(byte_capacity)?;
    let bytes = unsafe {
        libc::proc_listpids(
            PROCESS_GROUP_ONLY_FILTER,
            group as u32,
            processes.as_mut_ptr().cast(),
            byte_capacity,
        )
    };
    ensure!(bytes >= 0, "inspect retained process group members");
    ensure!(
        bytes < byte_capacity,
        "retained process group member limit exceeded"
    );
    let byte_count = usize::try_from(bytes)?;
    ensure!(
        byte_count % std::mem::size_of::<libc::pid_t>() == 0,
        "invalid retained process group member response"
    );
    let count = byte_count / std::mem::size_of::<libc::pid_t>();
    Ok(processes[..count]
        .iter()
        .any(|process| *process != 0 && *process != leader))
}
