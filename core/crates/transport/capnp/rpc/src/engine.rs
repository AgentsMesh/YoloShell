use crate::{api::*, connection::Call, constants::*};
use std::sync::{Arc, OnceLock};
use tokio::{
    sync::{OwnedSemaphorePermit, Semaphore, mpsc, oneshot, watch},
    time::{Instant, timeout_at},
};

pub(crate) const ADMISSION_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);
pub(crate) enum Job {
    Client {
        io: Box<dyn Duplex>,
        protocol: Protocol,
        calls: mpsc::Receiver<Call>,
        stopped: watch::Receiver<bool>,
    },
    Server {
        io: Box<dyn Duplex>,
        protocol: Protocol,
        service: SharedService,
        stopped: watch::Receiver<bool>,
        completion: oneshot::Sender<Result<(), Error>>,
        domain: tokio::runtime::Handle,
    },
}
struct Engine {
    sender: mpsc::Sender<(Job, OwnedSemaphorePermit)>,
    slots: Arc<Semaphore>,
}
static ENGINE: OnceLock<Result<Engine, Error>> = OnceLock::new();

pub(crate) async fn submit(job: Job, deadline: Instant) -> Result<(), Error> {
    let engine = ENGINE.get_or_init(start).as_ref().map_err(Clone::clone)?;
    let permit = engine
        .slots
        .clone()
        .try_acquire_owned()
        .map_err(|_| Error::Capacity)?;
    timeout_at(deadline, engine.sender.send((job, permit)))
        .await
        .map_err(|_| Error::Deadline)?
        .map_err(|_| Error::Runtime)
}
fn start() -> Result<Engine, Error> {
    let (sender, mut jobs) = mpsc::channel::<(Job, OwnedSemaphorePermit)>(ENGINE_QUEUE);
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|_| Error::Runtime)?;
    std::thread::Builder::new()
        .name(THREAD_NAME.into())
        .spawn(move || {
            runtime.block_on(tokio::task::LocalSet::new().run_until(async move {
                while let Some((job, permit)) = jobs.recv().await {
                    tokio::task::spawn_local(async move {
                        let _permit = permit;
                        match job {
                            Job::Client {
                                io,
                                protocol,
                                calls,
                                stopped,
                            } => crate::client::run(io, protocol, calls, stopped).await,
                            Job::Server {
                                io,
                                protocol,
                                service,
                                stopped,
                                completion,
                                domain,
                            } => {
                                let result =
                                    crate::server::run(io, protocol, service, stopped, domain)
                                        .await;
                                let _ = completion.send(result);
                            }
                        }
                    });
                }
            }));
        })
        .map_err(|_| Error::Runtime)?;
    Ok(Engine {
        sender,
        slots: Arc::new(Semaphore::new(MAX_CONNECTIONS)),
    })
}

pub(crate) fn options() -> capnp::message::ReaderOptions {
    capnp::message::ReaderOptions {
        traversal_limit_in_words: Some(yolo_capnp_codec::MAX_MESSAGE_BYTES / WORD_BYTES),
        nesting_limit: yolo_capnp_codec::CAPNP_NESTING_LIMIT,
    }
}
