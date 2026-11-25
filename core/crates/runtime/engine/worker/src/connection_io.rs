use super::*;

pub(super) async fn read_worker_frame<T>(stream: &mut UnixStream) -> Result<T>
where
    T: serde::de::DeserializeOwned,
{
    worker_operation_with_timeout(WORKER_OPERATION_TIMEOUT, async {
        Ok(read_frame(stream).await?)
    })
    .await
}

pub(super) async fn write_worker_frame<T>(stream: &mut UnixStream, value: &T) -> Result<()>
where
    T: Serialize,
{
    worker_operation_with_timeout(WORKER_OPERATION_TIMEOUT, async {
        write_frame(stream, value).await?;
        Ok(())
    })
    .await
}
