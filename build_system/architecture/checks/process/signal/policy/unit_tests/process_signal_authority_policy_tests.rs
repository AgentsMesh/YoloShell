use super::fixture::Fixture;
use super::process_signal_test_support::{messages, write_contract};

const FUTURE_PATH: &str = "core/crates/runtime/engine/process/src/future_authority.rs";
const AUTHORITY_PATH: &str = "core/crates/runtime/engine/process/src/signal/signal_authority.rs";
const CONSTRUCTOR_POLICY: &str = "authority constructor moved";
const ENTRY_POLICY: &str = "outside its private authority owner";
const TYPE_POLICY: &str = "authority type escaped";

#[test]
fn rejects_direct_and_indirect_authority_constructor_leaks() {
    let fixture = Fixture::new();
    write_contract(&fixture);
    fixture.write(
        FUTURE_PATH,
        r#"
fn leak() {
    let constructor = ProcessSignalAuthority::new;
    constructor();
}
"#,
    );
    let result = messages(&fixture);
    assert!(contains(&result, CONSTRUCTOR_POLICY));
    assert!(contains(&result, TYPE_POLICY));
}

#[test]
fn rejects_authorized_entry_moved_to_an_arbitrary_identity_owner() {
    let fixture = Fixture::new();
    write_contract(&fixture);
    fixture.write(
        FUTURE_PATH,
        "fn bypass() { arbitrary.signal_authorized(&authority, 9); }\n",
    );
    assert!(contains(&messages(&fixture), ENTRY_POLICY));
}

#[test]
fn rejects_authority_type_and_signal_call_moved_together() {
    let fixture = Fixture::new();
    write_contract(&fixture);
    fixture.write(
        FUTURE_PATH,
        r#"
fn bypass(authority: DestructiveSessionAuthority) {
    authority.signal(&arbitrary, 9);
}
"#,
    );
    let result = messages(&fixture);
    assert!(contains(&result, TYPE_POLICY));
    assert!(contains(&result, ENTRY_POLICY));
}

#[test]
fn rejects_verified_process_group_constructor_outside_scope_proof() {
    let fixture = Fixture::new();
    write_contract(&fixture);
    fixture.write(
        FUTURE_PATH,
        "fn forge() { ProcessGroupIdentity::verified(group, arbitrary); }\n",
    );
    let result = messages(&fixture);
    assert!(contains(&result, TYPE_POLICY));
    assert!(contains(&result, CONSTRUCTOR_POLICY));
}

#[test]
fn renamed_or_chained_authority_receivers_cannot_hide_signal_calls() {
    for call in [
        "permit.signal(&arbitrary, signal);",
        "source.permit().signal(&arbitrary, signal);",
    ] {
        let fixture = Fixture::new();
        write_contract(&fixture);
        fixture.write(FUTURE_PATH, &format!("fn bypass() {{ {call} }}\n"));
        assert!(contains(&messages(&fixture), ENTRY_POLICY));
    }
}

#[test]
fn import_and_type_aliases_cannot_hide_authority_constructors() {
    for source in [
        r#"
use crate::process::signal_authority::ProcessSignalAuthority as Permit;
fn bypass() { Permit::new(); }
"#,
        r#"
type Permit = ProcessSignalAuthority;
fn bypass() { Permit::new(); }
"#,
        r#"
use crate::process::signal_authority::ProcessSignalAuthority::{self as Permit};
fn bypass() { Permit::new(); }
"#,
    ] {
        let fixture = Fixture::new();
        write_contract(&fixture);
        fixture.write(FUTURE_PATH, source);
        let result = messages(&fixture);
        assert!(contains(&result, TYPE_POLICY));
        assert!(contains(&result, CONSTRUCTOR_POLICY));
    }
}

#[test]
fn angle_qualified_types_cannot_hide_authority_constructors() {
    for constructor in [
        "<ProcessSignalAuthority>::new()",
        "<crate::process::signal_authority::ProcessSignalAuthority>::new()",
    ] {
        let fixture = Fixture::new();
        write_contract(&fixture);
        fixture.write(FUTURE_PATH, &format!("fn bypass() {{ {constructor}; }}\n"));
        assert!(contains(&messages(&fixture), CONSTRUCTOR_POLICY));
    }
}

#[test]
fn authority_literals_cannot_bypass_constructor_sites() {
    for source in [
        "fn bypass() { ProcessSignalAuthority { _private: () }; }\n",
        "fn bypass() { ProcessSignalAuthority(()); }\n",
        "impl ProcessSignalAuthority { fn bypass() { Self { _private: () }; } }\n",
        "fn bypass() { ProcessGroupIdentity { id, members, authority }; }\n",
    ] {
        let fixture = Fixture::new();
        write_contract(&fixture);
        fixture.write(FUTURE_PATH, source);
        assert!(contains(&messages(&fixture), CONSTRUCTOR_POLICY));
    }
}

#[test]
fn self_new_resolves_to_the_enclosing_authority_owner() {
    let allowed = Fixture::new();
    write_contract(&allowed);
    allowed.write(
        AUTHORITY_PATH,
        "impl ProcessSignalAuthority { #[cfg(test)] fn for_test() -> Self { Self::new() } }\n",
    );
    assert_eq!(matches(&messages(&allowed), CONSTRUCTOR_POLICY), 1);

    let bypass = Fixture::new();
    write_contract(&bypass);
    bypass.write(
        AUTHORITY_PATH,
        "impl ProcessSignalAuthority { fn bypass() { Self::new(); } }\n",
    );
    assert_eq!(matches(&messages(&bypass), CONSTRUCTOR_POLICY), 2);
}

#[test]
fn associated_authority_signal_calls_obey_the_same_exact_sites() {
    let bypass = Fixture::new();
    write_contract(&bypass);
    bypass.write(
        AUTHORITY_PATH,
        r#"
impl ProcessSignalAuthority {
    fn bypass(authority: &Self) {
        ProcessSignalAuthority::signal(authority, identity, signal);
        Self::signal(authority, identity, signal);
    }
}
"#,
    );
    assert_eq!(matches(&messages(&bypass), ENTRY_POLICY), 2);

    let allowed = Fixture::new();
    write_contract(&allowed);
    allowed.write(
        AUTHORITY_PATH,
        "impl DestructiveSessionAuthority { fn signal(&self) { ProcessSignalAuthority::signal(&self.0, identity, signal); } }\n",
    );
    assert_eq!(matches(&messages(&allowed), ENTRY_POLICY), 0);
}

fn matches(messages: &[String], expected: &str) -> usize {
    messages
        .iter()
        .filter(|message| message.contains(expected))
        .count()
}

fn contains(messages: &[String], expected: &str) -> bool {
    messages.iter().any(|message| message.contains(expected))
}
