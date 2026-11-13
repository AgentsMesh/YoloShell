use super::*;

#[cfg(target_os = "macos")]
#[test]
fn environment_process_helper_entrypoint() {
    if std::env::var(ENVIRONMENT_HELPER_ROLE_ENV).as_deref() != Ok(ENVIRONMENT_HELPER_ROLE) {
        return;
    }
    let ready = required_environment_helper_path();
    std::fs::write(ready, ENVIRONMENT_HELPER_MARKER).unwrap();
    loop {
        std::thread::park();
    }
}
