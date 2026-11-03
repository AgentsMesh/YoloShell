use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt, DuplexStream};
use tokio::net::UnixStream;
use tokio::sync::{Notify, oneshot};
use tokio::task::JoinHandle;

const BUFFER_BYTES: usize = 64 * 1024;
const EMPTY_FRAGMENT_DETAIL: &str = "fragment must contain at least one RPC transport byte";

struct Pause {
    remaining: usize,
    reached: oneshot::Sender<()>,
    resume: Arc<Notify>,
}

pub(super) struct Transport {
    pause: Arc<Mutex<Option<Pause>>>,
    task: JoinHandle<std::io::Result<()>>,
}

impl Transport {
    pub(super) fn new(socket: UnixStream) -> (DuplexStream, Self) {
        let (server, bridge) = tokio::io::duplex(BUFFER_BYTES);
        let pause = Arc::new(Mutex::new(None));
        let outbound_pause = Arc::clone(&pause);
        let task = tokio::spawn(async move {
            let (mut server_read, mut server_write) = tokio::io::split(bridge);
            let (mut client_read, mut client_write) = socket.into_split();
            let inbound = tokio::io::copy(&mut client_read, &mut server_write);
            let outbound = async {
                let mut buffer = vec![0; BUFFER_BYTES];
                loop {
                    let count = server_read.read(&mut buffer).await?;
                    if count == 0 {
                        return client_write.shutdown().await;
                    }
                    forward(&mut client_write, &buffer[..count], &outbound_pause).await?;
                }
            };
            tokio::select! {
                result = inbound => result.map(|_| ()),
                result = outbound => result,
            }
        });
        (server, Self { pause, task })
    }

    pub(super) fn pause_after(
        &self,
        bytes: usize,
        reached: oneshot::Sender<()>,
        resume: Arc<Notify>,
    ) -> std::io::Result<()> {
        if bytes == 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                EMPTY_FRAGMENT_DETAIL,
            ));
        }
        assert!(
            self.pause
                .lock()
                .unwrap()
                .replace(Pause {
                    remaining: bytes,
                    reached,
                    resume,
                })
                .is_none()
        );
        Ok(())
    }
}

async fn forward(
    output: &mut (impl AsyncWriteExt + Unpin),
    bytes: &[u8],
    pause: &Mutex<Option<Pause>>,
) -> std::io::Result<()> {
    let (count, reached) = {
        let mut slot = pause.lock().unwrap();
        match slot.as_mut() {
            Some(pending) => {
                let count = bytes.len().min(pending.remaining);
                pending.remaining -= count;
                (
                    count,
                    (pending.remaining == 0).then(|| slot.take().unwrap()),
                )
            }
            None => (bytes.len(), None),
        }
    };
    output.write_all(&bytes[..count]).await?;
    if let Some(pending) = reached {
        let _ = pending.reached.send(());
        pending.resume.notified().await;
    }
    output.write_all(&bytes[count..]).await
}

impl Drop for Transport {
    fn drop(&mut self) {
        self.task.abort();
    }
}
