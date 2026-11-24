#[cfg(target_os = "macos")]
impl PtySessionProof {
    pub fn bind_secondary_custody_alias_for_test(
        &mut self,
        runtime: &crate::pty_custody::CustodyRuntimeIdentity,
    ) -> anyhow::Result<crate::process::PtyMasterAliasReleaseAuthority> {
        self.bind_runtime(runtime.terminal_id, runtime.runtime_epoch)?;
        self.bind_master_alias_release(ProcessIdentity {
            pid: runtime.child_pid,
            start_time: runtime.child_start_time,
        })
    }
}

#[cfg(target_os = "macos")]
#[doc(hidden)]
pub fn process_holds_capability_for_test(pid: u32, writer: u64) -> Result<bool> {
    macos_descriptor_capability::exact_descriptor_holder(pid, writer)
}
