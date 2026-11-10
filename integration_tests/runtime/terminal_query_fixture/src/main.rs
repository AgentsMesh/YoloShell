#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    yolo_terminal_query_fixture::run().await
}
