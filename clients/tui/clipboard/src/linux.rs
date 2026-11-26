use crate::files::{Item, file_urls};
use anyhow::{Context, Result, bail, ensure};
use tokio::process::Command;

#[path = "linux_constants.rs"]
mod constants;
pub(super) use constants::MIME_TYPE;
use constants::{IMAGES, LIST_TYPES, NO_NEWLINE, URI_LIST};

enum Clipboard {
    Wayland,
    X11,
}

impl Clipboard {
    async fn read(&self, kind: Option<&str>) -> Result<Vec<u8>> {
        let (program, args) = match (self, kind) {
            (Self::Wayland, None) => ("wl-paste", vec![LIST_TYPES]),
            (Self::Wayland, Some(kind)) => ("wl-paste", vec![NO_NEWLINE, MIME_TYPE, kind]),
            (Self::X11, kind) => (
                "xclip",
                vec![
                    "-selection",
                    "clipboard",
                    "-out",
                    "-target",
                    kind.unwrap_or("TARGETS"),
                ],
            ),
        };
        let output = Command::new(program)
            .args(args)
            .kill_on_drop(true)
            .output()
            .await
            .with_context(|| format!("Could not start {program} for this desktop clipboard"))?;
        ensure!(
            output.status.success(),
            "{program} could not read this desktop clipboard"
        );
        Ok(output.stdout)
    }
}

pub(super) async fn items() -> Result<Vec<Item>> {
    let mut error = None;
    for (display, backend) in [
        ("WAYLAND_DISPLAY", Clipboard::Wayland),
        ("DISPLAY", Clipboard::X11),
    ] {
        if std::env::var_os(display).is_none() {
            continue;
        }
        let types = match backend.read(None).await {
            Ok(types) => types,
            Err(failure) => {
                error = Some(failure);
                continue;
            }
        };
        let types = std::str::from_utf8(&types)?;
        if types.lines().any(|kind| kind == URI_LIST) {
            return file_urls(std::str::from_utf8(&backend.read(Some(URI_LIST)).await?)?);
        }
        for &(kind, name) in IMAGES {
            if types.lines().any(|value| value == kind) {
                let bytes = backend.read(Some(kind)).await?;
                ensure!(!bytes.is_empty(), "The clipboard image is empty");
                return Ok(vec![Item::Image { name, bytes }]);
            }
        }
        bail!("Copy a file or image, then paste it here");
    }
    Err(error.unwrap_or_else(|| {
        anyhow::anyhow!(
            "No desktop clipboard is available here. Ctrl-B u attaches a file from this machine"
        )
    }))
}
