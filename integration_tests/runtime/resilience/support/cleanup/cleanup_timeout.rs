use super::*;
use std::os::unix::net::UnixStream;

pub(super) fn configure(stream: &UnixStream) -> std::io::Result<()> {
    configure_with(
        stream,
        |stream, timeout| stream.set_read_timeout(Some(timeout)),
        |stream, timeout| stream.set_write_timeout(Some(timeout)),
    )
}

fn configure_with<T>(
    stream: &T,
    set_read: impl FnOnce(&T, Duration) -> std::io::Result<()>,
    set_write: impl FnOnce(&T, Duration) -> std::io::Result<()>,
) -> std::io::Result<()> {
    set_read(stream, CLEANUP_RPC_TIMEOUT)?;
    set_write(stream, CLEANUP_RPC_TIMEOUT)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TIMEOUT_FAILURE_DETAIL: &str = "injected cleanup timeout failure";

    #[test]
    fn read_failure_stops_configuration() {
        let write_called = std::cell::Cell::new(false);
        let result = configure_with(
            &(),
            |_, _| Err(std::io::Error::other(TIMEOUT_FAILURE_DETAIL)),
            |_, _| {
                write_called.set(true);
                Ok(())
            },
        );
        assert!(result.is_err());
        assert!(!write_called.get());
    }

    #[test]
    fn write_failure_is_propagated() {
        let result = configure_with(
            &(),
            |_, _| Ok(()),
            |_, _| Err(std::io::Error::other(TIMEOUT_FAILURE_DETAIL)),
        );
        assert!(result.is_err());
    }
}
