use crate::{api::*, constants::*, engine, server_events};
use capnp_rpc::{RpcSystem, rpc_twoparty_capnp::Side, twoparty};
use std::rc::Rc;
use std::sync::Arc;
use tokio::sync::{Semaphore, watch};
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};
use yolo_capnp_schema::{
    rpc_capnp::{endpoint, peer, result},
    value_capnp::value,
};

pub(crate) async fn run(
    io: Box<dyn Duplex>,
    protocol: Protocol,
    service: SharedService,
    mut stopped: watch::Receiver<bool>,
    domain: tokio::runtime::Handle,
) -> Result<(), Error> {
    let (read, write) = tokio::io::split(io);
    let network = twoparty::VatNetwork::new(
        read.compat(),
        write.compat_write(),
        Side::Server,
        engine::options(),
    );
    let scope = Arc::new(crate::domain::Scope::default());
    let confirmations = Rc::new(crate::confirmation_drain::Drain::default());
    let _scope = crate::domain::Guard(scope.clone());
    let server = Server {
        service,
        slots: Arc::new(Semaphore::new(MAX_IN_FLIGHT)),
        streams: Arc::new(Semaphore::new(MAX_STREAMS)),
        receipts: Arc::new(Semaphore::new(MAX_IN_FLIGHT)),
        domain,
        scope: scope.clone(),
        confirmations: confirmations.clone(),
    };
    let bootstrap = match protocol {
        Protocol::Endpoint => {
            let client: endpoint::Client = capnp_rpc::new_client(server);
            client.client
        }
        Protocol::Peer => {
            let client: peer::Client = capnp_rpc::new_client(server);
            client.client
        }
    };
    let system = RpcSystem::new(Box::new(network), Some(bootstrap));
    tokio::pin!(system);
    tokio::select! {
        _ = stopped.changed() => {
            scope.cancel();
            crate::server_shutdown::finish(system.as_mut(), confirmations).await
        },
        result = &mut system => result.map_err(Error::from),
    }
}
struct Server {
    service: SharedService,
    slots: Arc<Semaphore>,
    streams: Arc<Semaphore>,
    receipts: Arc<Semaphore>,
    domain: tokio::runtime::Handle,
    scope: Arc<crate::domain::Scope>,
    confirmations: Rc<crate::confirmation_drain::Drain>,
}
impl Server {
    async fn invoke(
        &self,
        method: Method,
        request: value::Reader<'_>,
    ) -> Result<Reply, capnp::Error> {
        let _permit = self
            .slots
            .clone()
            .try_acquire_owned()
            .map_err(|_| rpc_error(Error::Capacity))?;
        let value =
            yolo_capnp_codec::read_value(request).map_err(|error| rpc_error(error.into()))?;
        let service = self
            .service
            .upgrade()
            .ok_or_else(|| rpc_error(Error::Closed))?;
        let mut task = self
            .scope
            .spawn(
                &self.domain,
                async move { service.call(method, value).await },
            )
            .map_err(rpc_error)?;
        (&mut task.handle)
            .await
            .map_err(|error| capnp::Error::failed(error.to_string()))?
            .map_err(rpc_error)
    }
    fn fill(&self, reply: Reply, mut output: result::Builder<'_>) -> Result<(), capnp::Error> {
        yolo_capnp_codec::write_value(output.reborrow().init_value(), &reply.value)
            .map_err(|error| rpc_error(error.into()))?;
        if let Some(source) = reply.events {
            let permit = self
                .streams
                .clone()
                .try_acquire_owned()
                .map_err(|_| rpc_error(Error::Capacity))?;
            output.set_events(server_events::client(
                source,
                permit,
                self.domain.clone(),
                self.scope.clone(),
            ));
        }
        if let Some(callback) = reply.on_observed {
            let permit = self
                .receipts
                .clone()
                .try_acquire_owned()
                .map_err(|_| rpc_error(Error::Capacity))?;
            output.set_receipt(crate::receipt::client(
                callback,
                permit,
                self.domain.clone(),
                self.confirmations.clone(),
            ));
        }
        Ok(())
    }
}
impl endpoint::Server for Server {
    async fn call(
        self: capnp::capability::Rc<Self>,
        params: endpoint::CallParams,
        mut results: endpoint::CallResults,
    ) -> Result<(), capnp::Error> {
        let reply = self
            .invoke(Method::Call, params.get()?.get_request()?)
            .await?;
        self.fill(reply, results.get().init_result())
    }
}
impl peer::Server for Server {
    async fn discover(
        self: capnp::capability::Rc<Self>,
        params: peer::DiscoverParams,
        mut results: peer::DiscoverResults,
    ) -> Result<(), capnp::Error> {
        let reply = self
            .invoke(Method::Discover, params.get()?.get_request()?)
            .await?;
        self.fill(reply, results.get().init_result())
    }
    async fn execute(
        self: capnp::capability::Rc<Self>,
        params: peer::ExecuteParams,
        mut results: peer::ExecuteResults,
    ) -> Result<(), capnp::Error> {
        let reply = self
            .invoke(Method::Execute, params.get()?.get_request()?)
            .await?;
        self.fill(reply, results.get().init_result())
    }
}
