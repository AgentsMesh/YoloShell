const REDACTED_TEST_INPUT: &str = "[REDACTED]";

#[derive(Clone)]
pub(crate) struct TestInput(Vec<u8>);

impl TestInput {
    pub(crate) fn new(encoded: Vec<u8>) -> Self {
        Self(encoded)
    }

    pub(crate) fn as_slice(&self) -> &[u8] {
        self.0.as_slice()
    }
}

impl std::fmt::Debug for TestInput {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(REDACTED_TEST_INPUT)
    }
}
