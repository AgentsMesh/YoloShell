use super::super::constants::*;
use super::super::*;

const KEY_LINE: &str = "h ssh-ed25519 key\n";

fn comments_with_bytes(bytes: usize) -> String {
    let full_lines = bytes / MAX_KNOWN_HOSTS_LINE_BYTES;
    let remainder = bytes % MAX_KNOWN_HOSTS_LINE_BYTES;
    let mut contents = String::with_capacity(bytes);
    for _ in 0..full_lines {
        contents.push('#');
        contents.push_str(&"x".repeat(MAX_KNOWN_HOSTS_LINE_BYTES - 2));
        contents.push('\n');
    }
    if remainder != 0 {
        contents.push('#');
        contents.push_str(&"x".repeat(remainder - 1));
    }
    contents
}

#[test]
fn known_hosts_byte_limit_accepts_exactly_max_and_rejects_max_plus_one() {
    let exact = comments_with_bytes(MAX_KNOWN_HOSTS_BYTES);
    assert_eq!(exact.len(), MAX_KNOWN_HOSTS_BYTES);
    KnownHosts::parse(&exact).unwrap();
    let over = format!("{exact}x");
    assert!(matches!(
        KnownHosts::parse(&over),
        Err(RemoteError::LimitExceeded)
    ));
}

#[test]
fn known_hosts_line_limit_accepts_exactly_max_and_rejects_max_plus_one() {
    let exact = format!("#{}", "x".repeat(MAX_KNOWN_HOSTS_LINE_BYTES - 1));
    KnownHosts::parse(&exact).unwrap();
    assert!(matches!(
        KnownHosts::parse(&format!("{exact}x")),
        Err(RemoteError::LimitExceeded)
    ));
}

#[test]
fn known_hosts_entry_limit_accepts_exactly_max_and_rejects_max_plus_one() {
    let exact = KEY_LINE.repeat(MAX_KNOWN_HOSTS_ENTRIES);
    KnownHosts::parse(&exact).unwrap();
    assert!(matches!(
        KnownHosts::parse(&format!("{exact}{KEY_LINE}")),
        Err(RemoteError::LimitExceeded)
    ));
}

#[test]
fn known_hosts_counts_comment_and_blank_lines() {
    let exact = "#\n".repeat(MAX_KNOWN_HOSTS_LINES);
    KnownHosts::parse(&exact).unwrap();
    assert!(matches!(
        KnownHosts::parse(&format!("{exact}#\n")),
        Err(RemoteError::LimitExceeded)
    ));
}

#[test]
fn host_pattern_limit_accepts_exactly_max_and_rejects_max_plus_one() {
    let exact = vec!["h"; MAX_HOST_PATTERNS_PER_ENTRY].join(",");
    KnownHosts::parse(&format!("{exact} ssh-ed25519 key")).unwrap();
    let over = format!("{exact},h ssh-ed25519 key");
    assert!(matches!(
        KnownHosts::parse(&over),
        Err(RemoteError::LimitExceeded)
    ));
}

#[test]
fn endpoint_user_and_host_limits_are_enforced_before_matching() {
    RemoteEndpoint::standard("u".repeat(MAX_REMOTE_USER_BYTES), "h").unwrap();
    RemoteEndpoint::standard("u", "h".repeat(MAX_REMOTE_HOST_BYTES)).unwrap();
    assert!(RemoteEndpoint::standard("u".repeat(MAX_REMOTE_USER_BYTES + 1), "h").is_err());
    assert!(RemoteEndpoint::standard("u", "h".repeat(MAX_REMOTE_HOST_BYTES + 1)).is_err());
}
