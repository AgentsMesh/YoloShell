use std::{
    io,
    net::Shutdown,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};
use tokio::net::TcpStream;

pub(crate) struct SocketState {
    socket: std::net::TcpStream,
    closed: AtomicBool,
}
impl SocketState {
    pub fn close(&self) {
        if !self.closed.swap(true, Ordering::AcqRel) {
            let _ = self.socket.shutdown(Shutdown::Both);
        }
    }
    pub fn is_closed(&self) -> bool {
        self.closed.load(Ordering::Acquire)
    }
}
pub(crate) struct SocketOwner(pub Arc<SocketState>);
impl SocketOwner {
    pub fn capture(stream: TcpStream) -> io::Result<(TcpStream, Self)> {
        let original = stream.into_std()?;
        let retained = original.try_clone()?;
        let owner = Self(Arc::new(SocketState {
            socket: retained,
            closed: AtomicBool::new(false),
        }));
        Ok((TcpStream::from_std(original)?, owner))
    }
}
impl Drop for SocketOwner {
    fn drop(&mut self) {
        self.0.close();
    }
}
