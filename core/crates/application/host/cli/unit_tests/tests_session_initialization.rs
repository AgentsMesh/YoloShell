#[test]
fn only_detached_serve_requests_early_session_initialization() {
    let regular = Command::Serve {
        home: PathBuf::new(),
        socket: None,
        detached_session: false,
        startup_guard: false,
    };
    let detached = Command::Serve {
        home: PathBuf::new(),
        socket: None,
        detached_session: true,
        startup_guard: false,
    };
    assert!(!needs_detached_session(&regular));
    assert!(needs_detached_session(&detached));
    assert!(!needs_detached_session(&Command::BootstrapChild));
}
