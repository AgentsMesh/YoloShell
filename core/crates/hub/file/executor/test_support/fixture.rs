use crate::constants::*;
use std::path::PathBuf;
use yolo_hub_file_executor::FileExecutor;
use yolo_hub_file_helper_protocol::{
    Command, FILE_HELPER_SCHEMA, Request, RequestFields, RootBinding,
};
use yolo_hub_file_reader_test_support::{FILE_PATH, Fixture};

pub fn binary() -> PathBuf {
    PathBuf::from(std::env::var(HELPER_BINARY_ENV).unwrap())
        .canonicalize()
        .unwrap()
}
pub fn executor() -> FileExecutor {
    FileExecutor::new(binary()).unwrap()
}
pub fn inspect(fixture: &Fixture) -> Request {
    request(
        fixture,
        Command::Inspect {
            roots: vec![fixture.summary.clone()],
        },
    )
}
pub fn read(fixture: &Fixture, binding: RootBinding) -> Request {
    request(
        fixture,
        Command::Read {
            binding,
            request: fixture.request(FILE_PATH),
        },
    )
}
pub fn request(fixture: &Fixture, command: Command) -> Request {
    Request::new(RequestFields {
        schema_version: FILE_HELPER_SCHEMA,
        request_id: uuid::Uuid::new_v4(),
        origin: fixture.summary.fields().target.address.hub_id,
        protected: vec![fixture.private.to_str().unwrap().into()],
        command,
    })
    .unwrap()
}
pub fn deadline() -> tokio::time::Instant {
    tokio::time::Instant::now() + TEST_DEADLINE
}
