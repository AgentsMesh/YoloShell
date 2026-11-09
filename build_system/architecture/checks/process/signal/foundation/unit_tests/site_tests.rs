use yolo_architecture_policy_contracts::process_signal as contract;
use yolo_architecture_process_signal_foundation::process_signal_sites::{receiver_site, site};
use yolo_architecture_syntax::rust_source::RustSource;

#[test]
fn site_authority_requires_both_declared_owner_and_exact_function_and_path() {
    for owner in [None, Some(contract::PROCESS_SIGNAL_AUTHORITY)] {
        let declared = site(contract::SIGNAL_AUTHORITY_PATH, owner, contract::SIGNAL);
        let body = format!("fn {}() {{}}", contract::SIGNAL);
        let text = owner.map_or(body.clone(), |owner| format!("impl {owner} {{ {body} }}"));
        let source = RustSource::parse(&text);
        let function = source.functions().next().unwrap();
        assert!(declared.matches(contract::SIGNAL_AUTHORITY_PATH, Some(function)));
        assert!(!declared.matches(contract::MACOS_SIGNAL_PATH, Some(function)));
        assert!(!declared.matches(contract::SIGNAL_AUTHORITY_PATH, None));
        assert!(
            !site(contract::SIGNAL_AUTHORITY_PATH, owner, contract::VERIFIED)
                .matches(contract::SIGNAL_AUTHORITY_PATH, Some(function))
        );
        let receiver = receiver_site(
            contract::CHILD_RECEIVER,
            contract::SIGNAL_AUTHORITY_PATH,
            owner,
            contract::SIGNAL,
        );
        assert_eq!(receiver.receiver, contract::CHILD_RECEIVER);
        assert!(
            receiver
                .site
                .matches(contract::SIGNAL_AUTHORITY_PATH, Some(function))
        );
    }
}
