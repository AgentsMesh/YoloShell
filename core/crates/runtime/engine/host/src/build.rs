use std::sync::OnceLock;
use yolo_build_image::BuildImage;
use yolo_executable_identity::CapturedExecutable;

static IMAGE: OnceLock<anyhow::Result<CapturedExecutable>> = OnceLock::new();
const IMAGE_UNAVAILABLE: &str = "Host executable identity unavailable";

pub(super) fn current() -> anyhow::Result<BuildImage> {
    let captured = IMAGE
        .get_or_init(CapturedExecutable::capture)
        .as_ref()
        .map_err(|_| anyhow::anyhow!(IMAGE_UNAVAILABLE))?;
    Ok(captured.image()?.clone())
}
