#[path = "../test_support/real_schema_fixture.rs"]
mod fixture;
#[path = "real_schema_fixture_tests.rs"]
mod fixture_tests;
#[cfg(any(not(test), feature = "yolo-integration-tests"))] #[path = "../integration_tests/real_schema_integration_tests.rs"]
mod integration_tests;
