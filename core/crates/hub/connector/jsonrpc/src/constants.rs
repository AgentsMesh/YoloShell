pub const CONNECTOR_ERROR: i32 = -32002;
pub const METHOD_PREFIX: &str = "connector.";
pub(crate) const METHODS: &[&str] = &["hello", "list", "register", "activate", "publish", "revoke"];
pub(crate) const METHOD_FIELD: &str = "method";
pub(crate) const DESTINATION_FIELD: &str = "destination";
pub(crate) const OPERATION_ID_FIELD: &str = "operation_id";
pub(crate) const RESULT_TAG: &str = "result";
