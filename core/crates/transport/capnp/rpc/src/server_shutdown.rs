use crate::{Error, confirmation_drain::Drain, constants::*};
use capnp_rpc::{RpcSystem, rpc_twoparty_capnp::Side};
use std::{future::Future, pin::Pin, rc::Rc, task::Poll};

pub(crate) async fn finish(
    mut system: Pin<&mut RpcSystem<Side>>,
    drain: Rc<Drain>,
) -> Result<(), Error> {
    tokio::time::timeout(SERVER_DRAIN_TIMEOUT, async {
        tokio::select! {
            biased;
            result = &mut system => return result.map_err(Error::from),
            _ = drain.finish() => {},
        }
        // Cap'n Proto chains method completion and Results publication in the same poll.
        // Drive that publication before Disconnector enqueues Abort and its queue barrier.
        if let Some(result) = futures::future::poll_fn(|cx| {
            Poll::Ready(match system.as_mut().poll(cx) {
                Poll::Ready(result) => Some(result),
                Poll::Pending => None,
            })
        })
        .await
        {
            return result.map_err(Error::from);
        }
        let disconnect = system.get_disconnector();
        tokio::pin!(disconnect);
        tokio::select! {
            result = &mut system => result.map_err(Error::from),
            result = &mut disconnect => result.map_err(Error::from),
        }
    })
    .await
    .map_err(|_| Error::Deadline)?
}
