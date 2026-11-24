const MANAGEMENT_CLIENT_CLOSED_MESSAGE: &str = "management client closed";
const RUNTIME_PROXY_CLOSED_MESSAGE: &str = "runtime proxy closed";
const RUNTIME_SETUP_CLOSED_MESSAGE: &str = "runtime setup closed";
const TASK_ERROR_DETAIL_FALLBACK: &str = "broker task error formatting panicked";
const TASK_FUTURE_REQUIRED: &str = "broker task future guard is empty";

impl RemoteBroker {
    fn spawn_runtime(
        &self,
        stream: LocalUnixStream,
        clients: Arc<Semaphore>,
        attachments: Arc<Semaphore>,
        admission: Arc<BrokerAdmission>,
        setup_tasks: &BrokerTasks,
        proxy_tasks: Arc<BrokerTasks>,
    ) {
        if self.config.hub_transport() || !admission.is_adopted() {
            close_saturated(stream);
            return;
        }
        let Ok(permit) = clients.try_acquire_owned() else {
            close_saturated(stream);
            return;
        };
        let pool = Arc::clone(&self.pool);
        setup_tasks.spawn(async move {
            contain_task(async move {
                match setup_runtime_admitted(stream, pool, attachments).await {
                    Ok(Some(established)) => {
                        admission.promote(|| {
                            proxy_tasks.spawn(async move {
                                contain_task(async move {
                                    let _permit = permit;
                                    if let Err(error) = established.forward().await {
                                        report_task_error(error, RUNTIME_PROXY_CLOSED_MESSAGE);
                                    }
                                })
                                .await;
                            })
                        });
                    }
                    Ok(None) => {}
                    Err(error) => report_task_error(error, RUNTIME_SETUP_CLOSED_MESSAGE),
                }
            })
            .await;
        });
    }

    fn spawn_management(
        &self,
        stream: LocalUnixStream,
        clients: Arc<Semaphore>,
        admission: Arc<BrokerAdmission>,
        tasks: &BrokerTasks,
        hub: Arc<HubTasks>,
    ) {
        let Ok(permit) = clients.try_acquire_owned() else {
            close_saturated(stream);
            return;
        };
        let config = Arc::clone(&self.config);
        let pool = Arc::clone(&self.pool);
        let shutdown = Arc::clone(&self.shutdown);
        let retirement = Arc::clone(&self.retirement);
        tasks.spawn(async move {
            contain_task(async move {
                let _permit = permit;
                match manage_upgradable(
                    stream,
                    pool,
                    config,
                    shutdown,
                    retirement,
                    Arc::clone(&admission),
                    Arc::clone(&hub.capacity),
                )
                .await
                {
                    Ok(Some(established)) => {
                        drop(_permit);
                        admission.promote(|| hub.promote(established));
                    }
                    Ok(None) => {}
                    Err(error) => report_task_error(error, MANAGEMENT_CLIENT_CLOSED_MESSAGE),
                }
            })
            .await;
        });
    }
}

struct TaskFutureGuard<Value> {
    value: Option<Value>,
}

impl<Value> TaskFutureGuard<Value> {
    fn new(value: Value) -> Self {
        Self { value: Some(value) }
    }

    fn get_mut(&mut self) -> &mut Value {
        self.value.as_mut().expect(TASK_FUTURE_REQUIRED)
    }
}

impl<Value> Drop for TaskFutureGuard<Value> {
    fn drop(&mut self) {
        if let Some(value) = self.value.take() {
            discard_task_value(value);
        }
    }
}

async fn contain_task<Fut>(future: Fut)
where
    Fut: Future<Output = ()>,
{
    let _ = catch_task_future(future).await;
}

async fn catch_task_future<Fut>(future: Fut) -> std::result::Result<Fut::Output, ()>
where
    Fut: Future,
{
    let caught = AssertUnwindSafe(future).catch_unwind();
    let mut guarded = TaskFutureGuard::new(Box::pin(caught));
    match guarded.get_mut().as_mut().await {
        Ok(value) => Ok(value),
        Err(payload) => {
            dispose_task_panic(payload);
            Err(())
        }
    }
}

fn catch_task_value<Value>(operation: impl FnOnce() -> Value) -> std::result::Result<Value, ()> {
    match std::panic::catch_unwind(AssertUnwindSafe(operation)) {
        Ok(value) => Ok(value),
        Err(payload) => {
            dispose_task_panic(payload);
            Err(())
        }
    }
}

fn discard_task_value<Value>(value: Value) {
    let _ = catch_task_value(|| drop(value));
}

fn task_error_detail(error: anyhow::Error) -> String {
    let detail = catch_task_value(|| error.to_string())
        .unwrap_or_else(|()| TASK_ERROR_DETAIL_FALLBACK.to_owned());
    discard_task_value(error);
    detail
}

fn report_task_error(error: anyhow::Error, message: &'static str) {
    let detail = task_error_detail(error);
    let _ = catch_task_value(|| tracing::debug!(%detail, message = message));
}

fn dispose_task_panic(payload: Box<dyn std::any::Any + Send>) {
    if let Err(secondary) = std::panic::catch_unwind(AssertUnwindSafe(|| drop(payload))) {
        std::mem::forget(secondary);
    }
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../../unit_tests/server/server_task_containment_tests.rs"]
mod server_task_containment_tests;

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../../unit_tests/server/server_task_reporting_containment_tests.rs"]
mod server_task_reporting_containment_tests;
