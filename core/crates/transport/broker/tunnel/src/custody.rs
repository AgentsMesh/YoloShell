use futures::task::AtomicWaker;
use std::{
    io,
    os::fd::{AsRawFd, BorrowedFd, OwnedFd},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};
use yolo_local_unix::LocalUnixStream;

pub(crate) struct Custody {
    socket: OwnedFd,
    closed: AtomicBool,
    pub(crate) reader: AtomicWaker,
    pub(crate) writer: AtomicWaker,
}

#[derive(Clone)]
pub struct BrokerTunnelClose(pub(crate) Arc<Custody>);

impl BrokerTunnelClose {
    pub(crate) fn capture(stream: &LocalUnixStream) -> io::Result<Self> {
        // The source owns this descriptor throughout duplication; the copy prevents fd reuse.
        let socket = unsafe { BorrowedFd::borrow_raw(stream.as_raw_fd()) }.try_clone_to_owned()?;
        Ok(Self(Arc::new(Custody {
            socket,
            closed: AtomicBool::new(false),
            reader: AtomicWaker::new(),
            writer: AtomicWaker::new(),
        })))
    }
    pub fn close(&self) {
        if self.0.closed.swap(true, Ordering::AcqRel) {
            return;
        }
        // shutdown is nonblocking and affects only this connected socket, never its listener.
        unsafe {
            libc::shutdown(self.0.socket.as_raw_fd(), libc::SHUT_RDWR);
        }
        self.0.reader.wake();
        self.0.writer.wake();
    }
    pub(crate) fn closed(&self) -> bool {
        self.0.closed.load(Ordering::Acquire)
    }
}
