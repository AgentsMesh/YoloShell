use anyhow::{Context, Result};
use std::{
    io::Write,
    process::{Command, Stdio},
};
use yolo_process::ScopedWaitableChild;

const MISSING_SERVICE: &str = "startup parent fixture requires a service executable";
const SEPARATOR: &[u8] = b"\n";

#[tokio::main]
async fn main() -> Result<()> {
    let mut arguments = std::env::args_os().skip(1);
    let service = arguments.next().context(MISSING_SERVICE)?;
    let (sender, input) = yolo_startup_guard::channel()?;
    let mut command = Command::new(service);
    command
        .args(arguments)
        .stdin(Stdio::from(input))
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let child = ScopedWaitableChild::spawn_detached_before_deadline(
        &mut command,
        std::time::Instant::now() + yolo_startup_guard::COMMIT_TIMEOUT,
    )?;
    {
        let mut output = std::io::stdout().lock();
        serde_json::to_writer(&mut output, &child.process_id())?;
        output.write_all(SEPARATOR)?;
        output.flush()?;
    }
    std::future::pending::<()>().await;
    drop(sender);
    drop(child);
    Ok(())
}
