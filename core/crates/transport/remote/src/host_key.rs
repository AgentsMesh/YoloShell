use super::constants::{
    KNOWN_HOSTS_CA_MARKER, KNOWN_HOSTS_COMMENT_PREFIX, KNOWN_HOSTS_MARKER_PREFIX,
    KNOWN_HOSTS_REVOKED_MARKER, MAX_HOST_PATTERN_BYTES, MAX_HOST_PATTERNS_PER_ENTRY,
    MAX_KNOWN_HOSTS_BYTES, MAX_KNOWN_HOSTS_ENTRIES, MAX_KNOWN_HOSTS_LINE_BYTES,
    MAX_KNOWN_HOSTS_LINES,
};
use super::host_key_match::host_list_matches;
use super::{HostKey, RemoteEndpoint, RemoteError};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Marker {
    Trusted,
    Revoked,
    CertificateAuthority,
}

#[derive(Debug, Clone)]
struct Entry {
    marker: Marker,
    hosts: Vec<String>,
    key: HostKey,
}

#[derive(Debug, Clone, Default)]
pub struct KnownHosts {
    entries: Vec<Entry>,
}

impl KnownHosts {
    pub fn parse(contents: &str) -> Result<Self, RemoteError> {
        if contents.len() > MAX_KNOWN_HOSTS_BYTES {
            return Err(RemoteError::LimitExceeded);
        }
        let mut entries = Vec::new();
        for (index, raw) in contents.lines().enumerate() {
            if index == MAX_KNOWN_HOSTS_LINES {
                return Err(RemoteError::LimitExceeded);
            }
            let line_number = index + 1;
            if raw.len() > MAX_KNOWN_HOSTS_LINE_BYTES {
                return Err(RemoteError::LimitExceeded);
            }
            let line = raw.trim();
            if line.is_empty() || line.starts_with(KNOWN_HOSTS_COMMENT_PREFIX) {
                continue;
            }
            if entries.len() == MAX_KNOWN_HOSTS_ENTRIES {
                return Err(RemoteError::LimitExceeded);
            }
            entries.push(parse_entry(line, line_number)?);
        }
        Ok(Self { entries })
    }

    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, RemoteError> {
        Self::parse(&super::local_file::read_known_hosts(path.as_ref())?)
    }

    pub fn verify(&self, endpoint: &RemoteEndpoint, key: &HostKey) -> Result<(), RemoteError> {
        let host = endpoint.known_host_token();
        let mut host_matched = false;
        let mut trusted = false;
        let mut revoked = false;
        for entry in &self.entries {
            if entry.marker == Marker::CertificateAuthority
                || !host_list_matches(&entry.hosts, &host)
            {
                continue;
            }
            host_matched = true;
            if &entry.key == key {
                trusted |= entry.marker == Marker::Trusted;
                revoked |= entry.marker == Marker::Revoked;
            }
        }
        if revoked {
            Err(RemoteError::RevokedHostKey {
                host: endpoint.authority(),
            })
        } else if trusted {
            Ok(())
        } else if host_matched {
            Err(RemoteError::ChangedHostKey {
                host: endpoint.authority(),
            })
        } else {
            Err(RemoteError::UnknownHostKey {
                host: endpoint.authority(),
            })
        }
    }
}

fn parse_entry(line: &str, line_number: usize) -> Result<Entry, RemoteError> {
    let mut fields = line.split_whitespace();
    let first = fields
        .next()
        .ok_or(RemoteError::MalformedKnownHosts { line: line_number })?;
    let (marker, host_field) = if first.starts_with(KNOWN_HOSTS_MARKER_PREFIX) {
        let marker = match first {
            KNOWN_HOSTS_REVOKED_MARKER => Marker::Revoked,
            KNOWN_HOSTS_CA_MARKER => Marker::CertificateAuthority,
            _ => return Err(RemoteError::MalformedKnownHosts { line: line_number }),
        };
        (marker, next_field(&mut fields, line_number)?)
    } else {
        (Marker::Trusted, first)
    };
    let algorithm = next_field(&mut fields, line_number)?;
    let encoded = next_field(&mut fields, line_number)?;
    let mut hosts = Vec::new();
    for host in host_field.split(',') {
        if host.is_empty() {
            return Err(RemoteError::MalformedKnownHosts { line: line_number });
        }
        if host.len() > MAX_HOST_PATTERN_BYTES || hosts.len() == MAX_HOST_PATTERNS_PER_ENTRY {
            return Err(RemoteError::LimitExceeded);
        }
        hosts.push(host.to_owned());
    }
    let key = HostKey::new(algorithm, encoded)
        .map_err(|_| RemoteError::MalformedKnownHosts { line: line_number })?;
    Ok(Entry { marker, hosts, key })
}

fn next_field<'a>(
    fields: &mut impl Iterator<Item = &'a str>,
    line: usize,
) -> Result<&'a str, RemoteError> {
    fields
        .next()
        .ok_or(RemoteError::MalformedKnownHosts { line })
}
