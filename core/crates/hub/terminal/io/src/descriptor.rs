use crate::descriptor_group::{Registered, capture};
use std::{
    io,
    os::fd::{AsRawFd, RawFd},
    pin::Pin,
    task::{Context, Poll},
};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf, unix::AsyncFd};

pub struct Descriptor(AsyncFd<Registered>);

impl Descriptor {
    pub fn duplicate(source: RawFd) -> io::Result<Self> {
        let [descriptor] = Self::group([source])?;
        Ok(descriptor)
    }
    pub fn pair(first: RawFd, second: RawFd) -> io::Result<(Self, Self)> {
        let [first, second] = Self::group([first, second])?;
        Ok((first, second))
    }
    pub fn triple(first: RawFd, second: RawFd, third: RawFd) -> io::Result<(Self, Self, Self)> {
        let [first, second, third] = Self::group([first, second, third])?;
        Ok((first, second, third))
    }
    fn group<const N: usize>(sources: [RawFd; N]) -> io::Result<[Self; N]> {
        let descriptors = capture(&sources)?
            .into_iter()
            .map(|descriptor| AsyncFd::new(descriptor).map(Self))
            .collect::<io::Result<Vec<_>>>()?;
        descriptors
            .try_into()
            .map_err(|_| io::ErrorKind::InvalidData.into())
    }
}

impl AsyncRead for Descriptor {
    fn poll_read(
        self: Pin<&mut Self>,
        context: &mut Context<'_>,
        output: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        if output.remaining() == 0 {
            return Poll::Ready(Ok(()));
        }
        loop {
            let mut ready = std::task::ready!(self.0.poll_read_ready(context))?;
            let bytes = output.initialize_unfilled();
            let result = ready.try_io(|source| {
                let count = unsafe {
                    libc::read(source.as_raw_fd(), bytes.as_mut_ptr().cast(), bytes.len())
                };
                if count < 0 {
                    Err(io::Error::last_os_error())
                } else {
                    Ok(count as usize)
                }
            });
            match result {
                Ok(Ok(count)) => {
                    output.advance(count);
                    return Poll::Ready(Ok(()));
                }
                Ok(Err(error)) if error.kind() == io::ErrorKind::Interrupted => continue,
                Ok(Err(error)) => return Poll::Ready(Err(error)),
                Err(_) => continue,
            }
        }
    }
}

impl AsyncWrite for Descriptor {
    fn poll_write(
        self: Pin<&mut Self>,
        context: &mut Context<'_>,
        bytes: &[u8],
    ) -> Poll<io::Result<usize>> {
        if bytes.is_empty() {
            return Poll::Ready(Ok(0));
        }
        loop {
            let mut ready = std::task::ready!(self.0.poll_write_ready(context))?;
            let result = ready.try_io(|source| {
                let count =
                    unsafe { libc::write(source.as_raw_fd(), bytes.as_ptr().cast(), bytes.len()) };
                if count < 0 {
                    Err(io::Error::last_os_error())
                } else {
                    Ok(count as usize)
                }
            });
            match result {
                Ok(Err(error)) if error.kind() == io::ErrorKind::Interrupted => continue,
                Ok(result) => return Poll::Ready(result),
                Err(_) => continue,
            }
        }
    }
    fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
    fn poll_shutdown(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}
