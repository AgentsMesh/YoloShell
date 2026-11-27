//! Capture the actual executable image of this process using retained descriptors.
//! Serialized images contain only a bounded digest and size. They carry no PID,
//! pathname or credentials and are never authority to launch, replace or signal.
mod capture;
mod constants;
mod evidence;
mod fingerprint;
mod hash;
#[cfg(target_os = "macos")]
#[path = "platform_macos.rs"]
mod platform;
#[cfg(target_os = "linux")]
#[path = "platform_linux.rs"]
mod platform;

pub use capture::{CapturedExecutable, capture_current};
pub use yolo_build_image::BuildImage;

#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../integration_tests/lib.rs"]
mod integration_tests;
#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../unit_tests/lib.rs"]
mod unit_tests;
