use crate::PeerIo;
use std::{io, net::Shutdown};
use tokio::net::TcpStream;

pub(crate) struct SocketOwner(std::net::TcpStream);

impl SocketOwner {
    pub fn capture(stream: TcpStream) -> io::Result<PeerIo> {
        let original = stream.into_std()?;
        let retained = original.try_clone()?;
        let owner = Self(retained);
        Ok(PeerIo::new(TcpStream::from_std(original)?, move || {
            owner.close()
        }))
    }
    pub fn close(&self) {
        let _ = self.0.shutdown(Shutdown::Both);
    }
}

impl Drop for SocketOwner {
    fn drop(&mut self) {
        self.close();
    }
}
