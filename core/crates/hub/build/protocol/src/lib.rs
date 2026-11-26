use serde::{Deserialize, Serialize};
use yolo_build_image::BuildImage;

/// Images observed from the live services used by this Hub.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceBuild {
    pub host: BuildImage,
    pub hub: BuildImage,
    pub file_helper: BuildImage,
}

impl ServiceBuild {
    /// Returns whether every service was launched from one immutable image.
    ///
    /// A mutable CLI selector may resolve to a different release between two
    /// systemd restarts. Requiring one image here prevents a mixed Host/Hub
    /// generation from becoming ready and avoids attaching new state to an
    /// older service image.
    pub fn all_match(&self, expected: &BuildImage) -> bool {
        self.host == *expected && self.hub == *expected && self.file_helper == *expected
    }
}
