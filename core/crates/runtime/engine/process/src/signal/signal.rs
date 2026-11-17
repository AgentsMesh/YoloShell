use anyhow::{Result, bail};

pub fn parse_signal(name: &str) -> Result<i32> {
    #[cfg(unix)]
    {
        let upper = name.trim().to_ascii_uppercase();
        let normalized = upper.strip_prefix("SIG").unwrap_or(&upper);
        let signal = match normalized {
            "INT" => libc::SIGINT,
            "TERM" => libc::SIGTERM,
            "KILL" => libc::SIGKILL,
            "HUP" => libc::SIGHUP,
            "QUIT" => libc::SIGQUIT,
            "TSTP" => libc::SIGTSTP,
            "CONT" => libc::SIGCONT,
            "WINCH" => libc::SIGWINCH,
            _ => bail!("unsupported signal {name}"),
        };
        Ok(signal)
    }
    #[cfg(not(unix))]
    {
        let _ = name;
        bail!("signals are not supported on this platform")
    }
}
