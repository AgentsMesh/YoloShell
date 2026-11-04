use super::*;
use std::sync::{Condvar, Mutex};
use tokio::sync::Notify;

#[derive(Default)]
struct Gate {
    open: Mutex<bool>,
    condition: Condvar,
}
impl Gate {
    fn wait(&self) {
        let mut open = self.open.lock().unwrap();
        while !*open {
            open = self.condition.wait(open).unwrap();
        }
    }
    fn release(&self) {
        *self.open.lock().unwrap() = true;
        self.condition.notify_all();
    }
}
struct Release(Arc<Gate>);
impl Drop for Release {
    fn drop(&mut self) {
        self.0.release();
    }
}
struct Blocking {
    gate: Arc<Gate>,
    entered: Arc<Notify>,
}
impl Service for Blocking {
    fn call(&self, _: Method, value: Value) -> Future<'_, Reply> {
        Box::pin(async move {
            if value == Value::Null {
                self.entered.notify_one();
                self.gate.wait();
            }
            Ok(Reply::value(value))
        })
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn domain_mutex_blocking_does_not_stall_the_shared_rpc_executor() {
    let gate = Arc::new(Gate::default());
    let release = Release(gate.clone());
    let entered = Arc::new(Notify::new());
    let (client, server) = tokio::io::duplex(PIPE_BYTES);
    let serving = tokio::spawn(yolo_capnp_rpc::serve(
        server,
        Protocol::Endpoint,
        Arc::new(Blocking {
            gate,
            entered: entered.clone(),
        }),
    ));
    let client = Connection::open(client, Protocol::Endpoint, Instant::now() + TIMEOUT)
        .await
        .unwrap();
    let blocked = client.clone();
    let request = tokio::spawn(async move {
        blocked
            .request(Method::Call, Value::Null, Instant::now() + TIMEOUT)
            .await
    });
    tokio::time::timeout(TIMEOUT, entered.notified())
        .await
        .unwrap();
    let responsive = client
        .request(Method::Call, Value::Boolean(true), Instant::now() + TIMEOUT)
        .await
        .unwrap();
    assert_eq!(responsive.value, Value::Boolean(true));
    drop(release);
    assert!(request.await.unwrap().is_ok());
    drop(client);
    let _ = tokio::time::timeout(TIMEOUT, serving).await.unwrap();
}
