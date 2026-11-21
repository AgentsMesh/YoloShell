use super::build;
use tokio::runtime::RuntimeFlavor;

#[test]
fn terminal_relay_has_async_io_without_a_machine_sized_scheduler_pool() {
    let runtime = build().unwrap();
    assert_eq!(
        runtime.handle().runtime_flavor(),
        RuntimeFlavor::CurrentThread
    );
    runtime.block_on(async {
        let (mut writer, mut reader) = tokio::io::duplex(std::mem::size_of::<u8>());
        let sending = tokio::spawn(async move {
            tokio::io::AsyncWriteExt::write_u8(&mut writer, u8::MAX)
                .await
                .unwrap();
        });
        assert_eq!(
            tokio::io::AsyncReadExt::read_u8(&mut reader).await.unwrap(),
            u8::MAX
        );
        sending.await.unwrap();
        tokio::task::spawn_blocking(|| ()).await.unwrap();
        tokio::time::sleep(std::time::Duration::ZERO).await;
    });
}
