use anyhow::{Result, ensure};

const CURRENT_PROCESS: libc::pid_t = 0;
const SYSTEM_CALL_FAILURE: libc::pid_t = -1;
const SESSION_ID_CONTEXT: &str = "inspect detached session identity";
const SESSION_CREATE_CONTEXT: &str = "create detached process session";
const UNSUPPORTED_PLATFORM_CONTEXT: &str = "detached sessions are unsupported on this platform";

pub fn establish_detached_session() -> Result<()> {
    #[cfg(unix)]
    {
        let process_id = unsafe { libc::getpid() };
        let session_id = unsafe { libc::getsid(CURRENT_PROCESS) };
        ensure!(
            session_id != SYSTEM_CALL_FAILURE,
            "{SESSION_ID_CONTEXT}: {}",
            std::io::Error::last_os_error()
        );
        if session_id == process_id {
            return Ok(());
        }
        ensure!(
            unsafe { libc::setsid() } != SYSTEM_CALL_FAILURE,
            "{SESSION_CREATE_CONTEXT}: {}",
            std::io::Error::last_os_error()
        );
        Ok(())
    }
    #[cfg(not(unix))]
    {
        anyhow::bail!(UNSUPPORTED_PLATFORM_CONTEXT)
    }
}
