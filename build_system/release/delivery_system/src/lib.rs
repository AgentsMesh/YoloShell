pub use yolo_release_delivery_workflow::{DeliveryRequest, deliver};
pub use yolo_release_developer_release_system::{
    ACCEPTANCE_RECEIPT_ARGUMENT, DISTRIBUTE_OPERATION, DeveloperReleaseRequest,
    NOTARY_PROFILE_ARGUMENT, distribute,
};

#[cfg(all(
    test,
    any(
        feature = "yolo-delivery-tests",
        feature = "yolo-developer-release-tests"
    )
))]
pub(crate) use yolo_release_delivery_foundation::delivery_command;
#[cfg(all(
    test,
    any(
        feature = "yolo-delivery-tests",
        feature = "yolo-developer-release-tests"
    )
))]
pub(crate) use yolo_release_delivery_workflow::{delivery, delivery_receipt};
#[cfg(all(test, feature = "yolo-developer-release-tests"))]
pub(crate) use yolo_release_developer_release_system::{
    developer_release, developer_release_arguments, developer_release_command,
    developer_release_constants, developer_release_inventory,
};

#[cfg(all(test, feature = "yolo-delivery-tests"))]
#[path = "../integration_tests/delivery.rs"]
mod delivery_integration_tests;

#[cfg(all(test, feature = "yolo-developer-release-tests"))]
#[path = "../integration_tests/developer_release.rs"]
mod developer_release_integration_tests;
