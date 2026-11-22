#[cfg(unix)]
use super::{PTY_WRITER_DUPLICATE_CONTEXT, configure_nonblocking_pty};
use portable_pty::MasterPty;
use std::fs::File;
use std::io::{self, Write};
#[cfg(unix)]
use std::os::fd::FromRawFd;

#[cfg(unix)]
pub(crate) const MINIMUM_DUPLICATED_FD: libc::c_int = 0;
#[cfg(unix)]
pub(crate) const SYSCALL_FAILURE: libc::c_int = -1;

pub(crate) enum PtyInputWriterInner {
    Native(File),
    #[cfg(test)]
    Test(Box<dyn Write + Send>),
}

pub(crate) struct PtyInputWriter(PtyInputWriterInner);

impl PtyInputWriter {
    #[cfg(unix)]
    pub(crate) fn open(master: &dyn MasterPty) -> io::Result<Self> {
        let fd = configure_nonblocking_pty(master)?;
        let duplicate = unsafe { libc::fcntl(fd, libc::F_DUPFD_CLOEXEC, MINIMUM_DUPLICATED_FD) };
        if duplicate == SYSCALL_FAILURE {
            let error = io::Error::last_os_error();
            return Err(io::Error::new(
                error.kind(),
                format!("{PTY_WRITER_DUPLICATE_CONTEXT}: {error}"),
            ));
        }
        let file = unsafe { File::from_raw_fd(duplicate) };
        Ok(Self(PtyInputWriterInner::Native(file)))
    }

    #[cfg(not(unix))]
    pub(crate) fn open(_master: &dyn MasterPty) -> io::Result<Self> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            super::PTY_FD_UNAVAILABLE_DETAIL,
        ))
    }

    #[cfg(test)]
    pub(crate) fn test(writer: impl Write + Send + 'static) -> Self {
        Self(PtyInputWriterInner::Test(Box::new(writer)))
    }
}

impl Write for PtyInputWriter {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        match &mut self.0 {
            PtyInputWriterInner::Native(file) => file.write(data),
            #[cfg(test)]
            PtyInputWriterInner::Test(writer) => writer.write(data),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match &mut self.0 {
            PtyInputWriterInner::Native(file) => file.flush(),
            #[cfg(test)]
            PtyInputWriterInner::Test(writer) => writer.flush(),
        }
    }
}
