use yolo_build_image::{BuildImage, SHA256_HEX_BYTES};
use yolo_hub_build_protocol::ServiceBuild;

pub use crate::constants::*;

pub fn build() -> ServiceBuild {
    let image = BuildImage::new(DIGEST_CHARACTER.repeat(SHA256_HEX_BYTES), IMAGE_BYTES).unwrap();
    ServiceBuild {
        host: image.clone(),
        hub: image.clone(),
        file_helper: image,
    }
}
