use super::fixture::Fixture;
use super::process_signal_test_support::{messages, write_contract};

const AUTHORITY_PATH: &str = "core/crates/runtime/engine/process/src/signal/signal_authority.rs";
const CONSTRUCTOR_POLICY: &str = "authority constructor moved";
const FUTURE_PATH: &str = "core/crates/runtime/engine/process/src/future_authority.rs";

#[test]
fn linux_containment_authority_requires_exact_path_owner_and_method() {
    let allowed = Fixture::new();
    write_contract(&allowed);
    let allowed_messages = messages(&allowed);
    assert!(
        !contains(&allowed_messages, CONSTRUCTOR_POLICY),
        "{allowed_messages:#?}"
    );

    for (path, source) in [
        (
            FUTURE_PATH,
            "impl DestructiveSessionAccess { fn install_linux_containment() { DestructiveSessionAuthority(ProcessSignalAuthority::new()); } }\n",
        ),
        (
            AUTHORITY_PATH,
            "impl ForegroundSignalAuthority { fn install_linux_containment() { DestructiveSessionAuthority(ProcessSignalAuthority::new()); } }\n",
        ),
        (
            AUTHORITY_PATH,
            "impl DestructiveSessionAccess { fn install_linux_containment_bypass() { DestructiveSessionAuthority(ProcessSignalAuthority::new()); } }\n",
        ),
    ] {
        let fixture = Fixture::new();
        write_contract(&fixture);
        fixture.write(path, source);
        assert!(contains(&messages(&fixture), CONSTRUCTOR_POLICY));
    }
}

fn contains(messages: &[String], expected: &str) -> bool {
    messages.iter().any(|message| message.contains(expected))
}
