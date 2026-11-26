use super::*;
use crate::paths::directory::PrivateDirectory;

pub(in crate::host) struct CreationReceiptStore {
    pub(crate) token_index: ReceiptDirectory,
    pub(crate) session_index: ReceiptDirectory,
    pub(crate) staging: ReceiptDirectory,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum CreationReceiptLookup {
    Missing,
    Matched(CreationReceipt),
    Mismatched,
}

impl CreationReceiptStore {
    #[cfg(test)]
    pub(crate) fn open(home: &Path) -> Result<Self> {
        let home = ReceiptDirectory::open(home, "creation receipt home")?;
        Self::open_directory(home)
    }

    pub(in crate::host) fn open_retained(home: PrivateDirectory) -> Result<Self> {
        let home = ReceiptDirectory::from_private(home, "creation receipt home", false)?;
        Self::open_directory(home)
    }

    pub(crate) fn open_directory(home: ReceiptDirectory) -> Result<Self> {
        let root = home.ensure_child(CREATION_RECEIPT_STORE_DIRECTORY, "creation receipt store")?;
        let token_index = root.ensure_child(
            CREATION_RECEIPT_TOKEN_DIRECTORY,
            "creation receipt token index",
        )?;
        let session_index = root.ensure_child(
            CREATION_RECEIPT_SESSION_DIRECTORY,
            "creation receipt session index",
        )?;
        let staging = root.ensure_child(
            CREATION_RECEIPT_STAGING_DIRECTORY,
            "creation receipt staging",
        )?;
        cleanup_stale_receipts(&staging)?;
        Ok(Self {
            token_index,
            session_index,
            staging,
        })
    }

    pub(crate) fn lookup_token_digest(&self, digest: &str) -> Result<Option<CreationReceipt>> {
        anyhow::ensure!(
            valid_creation_digest(digest),
            "invalid receipt token lookup"
        );
        let receipt = read_receipt_at(&self.token_index, digest)?;
        if let Some(receipt) = &receipt {
            anyhow::ensure!(
                receipt.creation_token_digest == digest,
                "receipt token index mismatch"
            );
        }
        Ok(receipt)
    }

    pub(crate) fn lookup_session(&self, terminal_id: Uuid) -> Result<Option<CreationReceipt>> {
        anyhow::ensure!(!terminal_id.is_nil(), "nil receipt session lookup");
        let name = terminal_id.hyphenated().to_string();
        let receipt = read_receipt_at(&self.session_index, &name)?;
        if let Some(receipt) = &receipt {
            anyhow::ensure!(
                receipt.result.terminal_id == terminal_id,
                "receipt session index mismatch"
            );
        }
        Ok(receipt)
    }

    pub(crate) fn lookup_binding(
        &self,
        binding: &CreationBinding,
    ) -> Result<Option<CreationReceipt>> {
        match self.lookup_identity(&binding.creation_token_digest, binding.result)? {
            CreationReceiptLookup::Missing => Ok(None),
            CreationReceiptLookup::Matched(receipt) => {
                receipt.validate_binding_fingerprint(binding)?;
                Ok(Some(receipt))
            }
            CreationReceiptLookup::Mismatched => {
                anyhow::bail!("creation receipt binding mismatch")
            }
        }
    }

    pub(crate) fn lookup_identity(
        &self,
        digest: &str,
        result: CreateTerminalResult,
    ) -> Result<CreationReceiptLookup> {
        let by_token = self.lookup_token_digest(digest)?;
        let by_session = self.lookup_session(result.terminal_id)?;
        let receipt = match (by_token, by_session) {
            (None, None) => return Ok(CreationReceiptLookup::Missing),
            (Some(receipt), None) | (None, Some(receipt)) => receipt,
            (Some(left), Some(right)) if left == right => left,
            _ => return Ok(CreationReceiptLookup::Mismatched),
        };
        if receipt.creation_token_digest == digest && receipt.result == result {
            Ok(CreationReceiptLookup::Matched(receipt))
        } else {
            Ok(CreationReceiptLookup::Mismatched)
        }
    }

    pub(crate) fn publish(&self, receipt: &CreationReceipt) -> Result<()> {
        receipt.validate()?;
        publish_receipt_at(
            &self.staging,
            &self.token_index,
            &receipt.creation_token_digest,
            receipt,
        )?;
        publish_receipt_at(
            &self.staging,
            &self.session_index,
            &receipt.result.terminal_id.hyphenated().to_string(),
            receipt,
        )?;
        let installed =
            match self.lookup_identity(&receipt.creation_token_digest, receipt.result)? {
                CreationReceiptLookup::Matched(installed) => installed,
                CreationReceiptLookup::Missing => {
                    anyhow::bail!("published creation receipt disappeared")
                }
                CreationReceiptLookup::Mismatched => {
                    anyhow::bail!("published creation receipt changed")
                }
            };
        anyhow::ensure!(installed == *receipt, "published creation receipt changed");
        Ok(())
    }

    pub(crate) fn publish_before(
        &self,
        receipt: &CreationReceipt,
        deadline: HostOperationDeadline,
    ) -> std::result::Result<(), ErrorBody> {
        require_creation_budget(deadline)?;
        receipt.validate().map_err(internal)?;
        let publication = (|| -> Result<()> {
            publish_receipt_at_before(
                &self.staging,
                &self.token_index,
                &receipt.creation_token_digest,
                receipt,
                deadline,
            )?;
            publish_receipt_at_before(
                &self.staging,
                &self.session_index,
                &receipt.result.terminal_id.hyphenated().to_string(),
                receipt,
                deadline,
            )
        })();
        match publication {
            Err(error) if error.is::<CreationPublicationDeadlineExpired>() => {
                Err(creation_launch_uncertain())
            }
            result => result.map_err(internal),
        }
    }
}
