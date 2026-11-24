const DESTRUCTIVE_UNAVAILABLE_ENVIRONMENT: &str = "YOLO_TEST_DESTRUCTIVE_CAPABILITY_UNAVAILABLE";

pub(super) fn destructive_capability(available: bool) -> bool {
    available && std::env::var_os(DESTRUCTIVE_UNAVAILABLE_ENVIRONMENT).is_none()
}
