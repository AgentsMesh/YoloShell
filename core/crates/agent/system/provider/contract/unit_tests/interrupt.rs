use super::*;

const ESCAPE: &[u8] = b"\x1b";

#[test]
fn unsupported_is_the_default_and_delivers_nothing() {
    assert_eq!(InterruptDelivery::default(), InterruptDelivery::Unsupported);
    assert!(InterruptDelivery::Unsupported.unsupported());
    assert_eq!(InterruptDelivery::Unsupported.key(), None);
    assert_eq!(InterruptDelivery::Signal.key(), None);
    assert!(!InterruptDelivery::Signal.unsupported());
}

#[test]
fn key_delivery_returns_the_provider_interrupt_key() {
    assert_eq!(InterruptDelivery::Key(ESCAPE).key(), Some(ESCAPE));
    assert!(!InterruptDelivery::Key(ESCAPE).unsupported());
}
