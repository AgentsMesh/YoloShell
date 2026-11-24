use super::super::identity_passphrase_constants::*;
use super::{
    MAX_IDENTITY_FILES, MAX_PASSPHRASE_BYTES,
    wire::{Envelope, SecretText},
};
use std::collections::BTreeMap;
use yolo_capnp_codec::ValueError;
use yolo_capnp_schema::value_capnp::value;
use yolo_capnp_sensitive::invalid;
use zeroize::Zeroizing;

/// Decode secret text directly from borrowed Cap'n Proto nodes, without serde
/// intermediary String owners or payload-bearing error messages.
pub(super) fn decode(bytes: &[u8]) -> Result<Envelope, ValueError> {
    yolo_capnp_sensitive::read(bytes, |root| {
        let value::Map(entries) = root.which().map_err(|_| invalid())? else {
            return Err(invalid());
        };
        let entries = entries.map_err(|_| invalid())?;
        if entries.len() as usize != ENVELOPE_FIELD_COUNT {
            return Err(invalid());
        }
        let mut fields = BTreeMap::new();
        for entry in entries.iter() {
            let key = text(entry.get_key().map_err(|_| invalid())?)?;
            if ![
                VERSION_FIELD,
                BINDING_NONCE_FIELD,
                HOST_ID_FIELD,
                CREDENTIAL_REVISION_FIELD,
                IDENTITY_PATHS_DIGEST_FIELD,
                PASSPHRASES_FIELD,
            ]
            .contains(&key)
                || fields
                    .insert(key, entry.get_value().map_err(|_| invalid())?)
                    .is_some()
            {
                return Err(invalid());
            }
        }
        let get = |key| fields.get(key).copied().ok_or_else(invalid);
        let value::Unsigned(version) = get(VERSION_FIELD)?.which().map_err(|_| invalid())? else {
            return Err(invalid());
        };
        let revision = get(CREDENTIAL_REVISION_FIELD)?;
        let identity_credential_revision =
            if matches!(revision.which().map_err(|_| invalid())?, value::Null(())) {
                None
            } else {
                Some(text(revision)?.to_owned())
            };
        let value::List(passphrases) = get(PASSPHRASES_FIELD)?.which().map_err(|_| invalid())?
        else {
            return Err(invalid());
        };
        let passphrases = passphrases.map_err(|_| invalid())?;
        if passphrases.len() as usize > MAX_IDENTITY_FILES {
            return Err(invalid());
        }
        let passphrases = passphrases
            .iter()
            .map(|item| {
                if matches!(item.which().map_err(|_| invalid())?, value::Null(())) {
                    return Ok(None);
                }
                let value = text(item)?;
                if value.is_empty() || value.len() > MAX_PASSPHRASE_BYTES {
                    return Err(invalid());
                }
                Ok(Some(SecretText(Zeroizing::new(value.to_owned()))))
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Envelope {
            version: version.try_into().map_err(|_| invalid())?,
            binding_nonce: text(get(BINDING_NONCE_FIELD)?)?.to_owned(),
            host_id: text(get(HOST_ID_FIELD)?)?.to_owned(),
            identity_credential_revision,
            identity_paths_sha256: text(get(IDENTITY_PATHS_DIGEST_FIELD)?)?.to_owned(),
            passphrases,
        })
    })
}

fn text<'a>(node: value::Reader<'a>) -> Result<&'a str, ValueError> {
    let value::Text(value) = node.which().map_err(|_| invalid())? else {
        return Err(invalid());
    };
    value
        .map_err(|_| invalid())?
        .to_str()
        .map_err(|_| invalid())
}
