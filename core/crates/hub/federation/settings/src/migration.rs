use crate::{Configuration, ExportPolicy, Peer, PeerAddress};
use serde::{Deserialize, Serialize, de::IgnoredAny};
use std::num::NonZeroU16;
use yolo_hub_delegation::PeerIdentity;

/// 0.1.x configurations carried a TCP `listen` field and TCP/loopback-peer
/// dial addresses. The listener is now derived from Hub state and dials are
/// Unix sockets behind SSH, so legacy dials are dropped while peer pins,
/// consume flags and export policy survive. Re-running remote setup restores
/// the dial on the consuming side.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ConfigurationWire {
    #[serde(default, rename = "listen", deserialize_with = "ignore_any")]
    _listen: (),
    peers: Vec<PeerWire>,
    trusted_origins: Vec<PeerIdentity>,
    file_roots: Vec<yolo_hub_file_protocol::FileRootConfiguration>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PeerWire {
    identity: PeerIdentity,
    dial: Option<PeerDialWire>,
    consume: bool,
    exports: Vec<ExportPolicy>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum PeerDialWire {
    Current(PeerAddress),
    Legacy(LegacyPeerAddress),
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacyPeerAddress {
    #[allow(dead_code)]
    host: String,
    #[allow(dead_code)]
    port: NonZeroU16,
    #[allow(dead_code)]
    transport: LegacyTransport,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum LegacyTransport {
    Tcp {},
    Ssh {
        #[allow(dead_code)]
        profile: crate::SshProfileRef,
    },
}

fn ignore_any<'de, D>(deserializer: D) -> Result<(), D::Error>
where
    D: serde::Deserializer<'de>,
{
    IgnoredAny::deserialize(deserializer)?;
    Ok(())
}

impl TryFrom<ConfigurationWire> for Configuration {
    type Error = anyhow::Error;
    fn try_from(wire: ConfigurationWire) -> anyhow::Result<Self> {
        Ok(Self {
            peers: wire.peers.into_iter().map(Peer::from).collect(),
            trusted_origins: wire.trusted_origins,
            file_roots: wire.file_roots,
        })
    }
}

impl From<PeerWire> for Peer {
    fn from(wire: PeerWire) -> Self {
        Self {
            identity: wire.identity,
            dial: match &wire.dial {
                Some(PeerDialWire::Current(address)) => Some(address.clone()),
                Some(PeerDialWire::Legacy(_)) | None => None,
            },
            // Importing requires a dial. A retired legacy dial cannot be
            // silently reused, so its import intent is dropped as well.
            consume: wire.consume
                && matches!(wire.dial, Some(PeerDialWire::Current(_))),
            exports: wire.exports,
        }
    }
}

/// Serialization shape intentionally omits the retired `listen` field.
#[derive(Serialize)]
pub(crate) struct ConfigurationOut {
    peers: Vec<PeerOut>,
    trusted_origins: Vec<PeerIdentity>,
    file_roots: Vec<yolo_hub_file_protocol::FileRootConfiguration>,
}

#[derive(Serialize)]
struct PeerOut {
    identity: PeerIdentity,
    dial: Option<PeerAddress>,
    consume: bool,
    exports: Vec<ExportPolicy>,
}

impl From<Configuration> for ConfigurationOut {
    fn from(value: Configuration) -> Self {
        Self {
            peers: value
                .peers
                .into_iter()
                .map(|peer| PeerOut {
                    identity: peer.identity,
                    dial: peer.dial,
                    consume: peer.consume,
                    exports: peer.exports,
                })
                .collect(),
            trusted_origins: value.trusted_origins,
            file_roots: value.file_roots,
        }
    }
}
