use crate::{
    Dialog,
    dialog::Source,
    files::{Item, ReferenceStyle},
    platform,
};
use anyhow::{Context, Result, ensure};
use std::{future::Future, pin::Pin, time::Duration};
use tokio::{fs::OpenOptions, time::Instant};
use yolo_hub_client::HubEndpoint;
use yolo_hub_identity::{HubId, RuntimeTarget};

type Transfer = Pin<Box<dyn Future<Output = Result<String>> + Send>>;
const IDLE_TIMEOUT: Duration = Duration::from_secs(30);

/// The UI keeps driving its PTY and keyboard while this file is in flight.
/// Dropping the future closes the transfer connection and any clipboard reader.
#[derive(Default)]
pub struct Transfers(Option<Transfer>);

impl Transfers {
    pub fn sync(&mut self, dialog: Option<&mut Dialog>, endpoint: &HubEndpoint, hub: HubId) {
        let Some(dialog) = dialog else {
            self.0 = None;
            return;
        };
        if let Some(source) = dialog.take_source() {
            self.0 = Some(Box::pin(upload(
                endpoint.clone(),
                hub,
                dialog.target,
                source,
                dialog.style,
            )));
        }
    }

    pub async fn next(&mut self) -> Result<String> {
        let Some(current) = &mut self.0 else {
            return std::future::pending().await;
        };
        let result = current.await;
        self.0 = None;
        result
    }
}

async fn upload(
    endpoint: HubEndpoint,
    hub: HubId,
    target: RuntimeTarget,
    source: Source,
    style: ReferenceStyle,
) -> Result<String> {
    let items = match source {
        Source::Clipboard => platform::items().await?,
        Source::File(path) => vec![Item::File(path)],
    };
    ensure!(
        !items.is_empty(),
        "No file or image was found on the clipboard"
    );
    let mut client = endpoint.connect(hub, Instant::now() + IDLE_TIMEOUT).await?;
    let mut paths = Vec::new();
    for item in items {
        let path = match item {
            Item::File(path) => {
                let mut file = OpenOptions::new()
                    .read(true)
                    .custom_flags(libc::O_NONBLOCK)
                    .open(&path)
                    .await
                    .with_context(|| format!("Open {}", path.display()))?;
                ensure!(
                    file.metadata().await?.is_file(),
                    "Choose a regular file: {}",
                    path.display()
                );
                let name = path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .context("File name is not valid UTF-8")?;
                client
                    .stage_attachment(target, name, &mut file, IDLE_TIMEOUT)
                    .await?
            }
            Item::Image { name, bytes } => {
                client
                    .stage_attachment(target, name, &mut bytes.as_slice(), IDLE_TIMEOUT)
                    .await?
            }
        };
        paths.push(path);
    }
    style.format(&paths)
}
