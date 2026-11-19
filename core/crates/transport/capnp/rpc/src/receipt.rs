use crate::{Confirmation, Error, constants::RECEIPT_EXPIRED};
use futures::{
    FutureExt,
    future::{BoxFuture, Shared},
};
use std::{cell::RefCell, rc::Rc};
use tokio::{
    sync::OwnedSemaphorePermit,
    time::{Instant, timeout_at},
};
use yolo_capnp_schema::rpc_capnp::{receipt, result};

pub(crate) use super::receipt_client::{ConfirmRequest, DeferredReceipt, defer};
type Ticket = (Box<dyn Confirmation>, OwnedSemaphorePermit);
type Completion = Shared<BoxFuture<'static, Result<(), Error>>>;

pub(crate) fn client(
    callback: Box<dyn Confirmation>,
    permit: OwnedSemaphorePermit,
    domain: tokio::runtime::Handle,
    drain: Rc<crate::confirmation_drain::Drain>,
) -> receipt::Client {
    let pending = Rc::new(RefCell::new(Some((callback, permit))));
    let expiring = Rc::downgrade(&pending);
    let expiry = tokio::task::spawn_local(async move {
        tokio::time::sleep(crate::MAX_RECEIPT_WAIT).await;
        if let Some(pending) = expiring.upgrade() {
            pending.borrow_mut().take();
        }
    });
    capnp_rpc::new_client(Receipt {
        pending,
        completion: RefCell::new(None),
        domain,
        expiry,
        drain,
    })
}
struct Receipt {
    pending: Rc<RefCell<Option<Ticket>>>,
    completion: RefCell<Option<Completion>>,
    domain: tokio::runtime::Handle,
    expiry: tokio::task::JoinHandle<()>,
    drain: Rc<crate::confirmation_drain::Drain>,
}
impl Drop for Receipt {
    fn drop(&mut self) {
        self.expiry.abort();
    }
}
impl receipt::Server for Receipt {
    async fn confirm(
        self: capnp::capability::Rc<Self>,
        _: receipt::ConfirmParams,
        _: receipt::ConfirmResults,
    ) -> Result<(), capnp::Error> {
        let _confirming = self.drain.begin();
        let existing = self.completion.borrow().clone();
        let completion = if let Some(existing) = existing {
            existing
        } else {
            let (callback, permit) = self
                .pending
                .borrow_mut()
                .take()
                .ok_or_else(|| capnp::Error::failed(RECEIPT_EXPIRED.into()))?;
            self.expiry.abort();
            let running = self.domain.spawn(async move {
                let _permit = permit;
                tokio::time::timeout(crate::MAX_RECEIPT_WAIT, callback.confirm())
                    .await
                    .map_err(|_| Error::OutcomeUnknown)?
            });
            let completion = async move { running.await.map_err(|_| Error::OutcomeUnknown)? }
                .boxed()
                .shared();
            self.completion.replace(Some(completion.clone()));
            completion
        };
        completion.await.map_err(crate::api::rpc_error)
    }
}

/// A reply carrying a commit receipt is not a certain business success until confirmation succeeds.
pub(crate) async fn confirm(result: result::Reader<'_>, deadline: Instant) -> Result<(), Error> {
    if !result.has_receipt() {
        return Ok(());
    }
    let receipt = result.get_receipt().map_err(|_| Error::OutcomeUnknown)?;
    timeout_at(deadline, receipt.confirm_request().send().promise)
        .await
        .map_err(|_| Error::OutcomeUnknown)?
        .map_err(|_| Error::OutcomeUnknown)
        .map(|_| ())
}
