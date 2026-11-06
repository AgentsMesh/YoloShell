pub(super) const CURRENT_SOURCE: &str = r#"
    fn resume_session() { perform_current_resume(); }
    use current::{TerminalTerminateRequest, FencedTerminateRequest};
    const RESUME_SESSION_EXACT: &str = "resume_session_exact";
    const WRITER_LEASE_V2: &str = "writer_lease_v2";
    const GET_WRITER_LEASE: &str = "get_writer_lease";
    // Reject "detach_session" and TerminateRequest.
    /* "ack_output" /* nested "resume_session" */ WRITER_LEASE */
    const ERROR: &str = "unsupported old method resume_session";
"#;
