use super::delivery_fixture::DeliveryFixture;
use super::developer_release_fixture_archive::candidate_archive;
pub use super::developer_release_fixture_runner::{
    FakeDeveloperRunner, INJECTED_ASSESSMENT_FAILURE, INJECTED_NOTARY_FAILURE,
    POSTPUBLICATION_ASSESSMENT_CALL,
};
use crate::developer_release::DeveloperReleaseRequest;

pub const TEST_NOTARY_PROFILE: &str = "yolo-release-test";

pub struct DeveloperReleaseFixture {
    pub delivery: DeliveryFixture,
}

impl DeveloperReleaseFixture {
    pub fn new() -> Self {
        Self {
            delivery: DeliveryFixture::from_archive(candidate_archive()),
        }
    }

    pub fn request(&self) -> DeveloperReleaseRequest<'_> {
        DeveloperReleaseRequest {
            candidate_directory: &self.delivery.candidate,
            output_directory: &self.delivery.delivery,
            notary_profile: TEST_NOTARY_PROFILE,
            acceptance_receipt: None,
        }
    }
}
