use std::{net::SocketAddr, time::Duration};
use yolo_web_gateway::*;

mod configuration;

const TOKEN: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

#[test]
fn origins_are_exact_authorities_and_cover_ipv6() {
    for value in [
        "https://example.test",
        "http://127.0.0.1:3000",
        "http://[::1]:3000",
    ] {
        assert_eq!(WebOrigin::parse(value).unwrap().as_str(), value);
    }
    assert_eq!(
        WebOrigin::parse("HTTPS://Example.test").unwrap().as_str(),
        "https://example.test"
    );
    for value in [
        "null",
        "https://example.test/path",
        "https://example.test/",
        "https://a@evil.test",
        "https://example.test?x",
        "https://example.test#x",
        "https://example.test:99999",
        "https://example.test\\@evil.test",
    ] {
        assert!(WebOrigin::parse(value).is_err(), "{value}");
    }
}

#[test]
fn tokens_require_fixed_size_and_never_render_secrets() {
    let token = AccessToken::from_hex(TOKEN).unwrap();
    assert!(token.matches(TOKEN));
    assert!(!token.matches(&"f".repeat(TOKEN_HEX_BYTES)));
    assert!(!format!("{token:?}").contains(TOKEN));
    for value in [
        "",
        "not-a-token",
        &"z".repeat(TOKEN_HEX_BYTES),
        &"A".repeat(TOKEN_HEX_BYTES),
    ] {
        assert!(AccessToken::from_hex(value).is_err());
    }
}

#[test]
fn accept_retries_only_per_connection_failures() {
    use std::io::{Error, ErrorKind};
    for kind in [
        ErrorKind::Interrupted,
        ErrorKind::WouldBlock,
        ErrorKind::ConnectionAborted,
        ErrorKind::ConnectionReset,
        ErrorKind::TimedOut,
    ] {
        assert!(accept_error_is_transient(&Error::from(kind)), "{kind:?}");
    }
    for kind in [
        ErrorKind::InvalidInput,
        ErrorKind::NotConnected,
        ErrorKind::PermissionDenied,
        ErrorKind::AddrInUse,
        ErrorKind::BrokenPipe,
    ] {
        assert!(!accept_error_is_transient(&Error::from(kind)), "{kind:?}");
    }
}

#[test]
fn gateway_cannot_expose_a_plaintext_non_loopback_listener() {
    let make = |bind: SocketAddr, lifetime, capacity| {
        GatewayConfig::new(
            bind,
            [WebOrigin::parse("https://example.test").unwrap()],
            AccessToken::from_hex(TOKEN).unwrap(),
            lifetime,
            capacity,
        )
    };
    assert!(
        make(
            "127.0.0.1:0".parse().unwrap(),
            DEFAULT_SESSION_LIFETIME,
            DEFAULT_MAX_CONNECTIONS
        )
        .is_ok()
    );
    assert!(
        make(
            "0.0.0.0:0".parse().unwrap(),
            DEFAULT_SESSION_LIFETIME,
            DEFAULT_MAX_CONNECTIONS
        )
        .is_err()
    );
    assert!(
        make(
            "127.0.0.1:0".parse().unwrap(),
            Duration::ZERO,
            DEFAULT_MAX_CONNECTIONS
        )
        .is_err()
    );
    assert!(make("127.0.0.1:0".parse().unwrap(), DEFAULT_SESSION_LIFETIME, 0).is_err());
}
