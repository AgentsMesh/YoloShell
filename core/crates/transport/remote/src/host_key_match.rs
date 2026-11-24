use super::constants::{HASHED_HOST_PREFIX, HASHED_HOST_SEPARATOR, NEGATED_HOST_PREFIX};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use hmac::{Hmac, Mac};
use sha1::Sha1;

pub(super) fn host_list_matches(patterns: &[String], host: &str) -> bool {
    let mut positive = false;
    for raw in patterns {
        let (negated, pattern) = raw
            .strip_prefix(NEGATED_HOST_PREFIX)
            .map_or((false, raw.as_str()), |value| (true, value));
        let matched = host_matches(host, pattern);
        if negated && matched {
            return false;
        }
        positive |= !negated && matched;
    }
    positive
}

fn host_matches(host: &str, pattern: &str) -> bool {
    if pattern.starts_with(HASHED_HOST_PREFIX) {
        return hashed_host_matches(host, pattern);
    }
    wildcard_matches(host.as_bytes(), pattern.as_bytes())
}

fn hashed_host_matches(host: &str, pattern: &str) -> bool {
    let Some(encoded) = pattern.strip_prefix(HASHED_HOST_PREFIX) else {
        return false;
    };
    let mut parts = encoded.split(HASHED_HOST_SEPARATOR);
    let decoded = parts
        .next()
        .and_then(|salt| STANDARD.decode(salt).ok())
        .zip(parts.next().and_then(|hash| STANDARD.decode(hash).ok()));
    let Some((salt, hash)) = decoded else {
        return false;
    };
    Hmac::<Sha1>::new_from_slice(&salt)
        .map(|mut value| {
            value.update(host.as_bytes());
            value.verify_slice(&hash).is_ok()
        })
        .unwrap_or(false)
}

fn wildcard_matches(value: &[u8], pattern: &[u8]) -> bool {
    let (mut value_at, mut pattern_at, mut star, mut retry) = (0, 0, None, 0);
    while value_at < value.len() {
        if pattern.get(pattern_at) == Some(&b'?')
            || pattern.get(pattern_at).map(u8::to_ascii_lowercase)
                == Some(value[value_at].to_ascii_lowercase())
        {
            value_at += 1;
            pattern_at += 1;
        } else if pattern.get(pattern_at) == Some(&b'*') {
            star = Some(pattern_at);
            pattern_at += 1;
            retry = value_at;
        } else if let Some(star_at) = star {
            retry += 1;
            value_at = retry;
            pattern_at = star_at + 1;
        } else {
            return false;
        }
    }
    pattern[pattern_at..].iter().all(|byte| *byte == b'*')
}
