use crate::{
    constants::*,
    fields::{Field, FieldId},
    form::{Form, FormKind},
    model::Action,
    wire_fields::{raw_text, text},
};
use anyhow::{Context, Result, ensure};
use yolo_hub_federation_settings::Configuration;
use yolo_hub_file_protocol::{FileRootConfiguration, FileRootFields};
use yolo_hub_identity::FileRootId;

impl Form {
    pub fn file_root(root: Option<&FileRootConfiguration>) -> Self {
        let id = root.map(|root| root.fields().root_id).unwrap_or_default();
        let mut form = Self::new(
            FormKind::FileRoot {
                id,
                existing: root.is_some(),
            },
            vec![
                Field::text(FieldId::RootId, id.to_string(), true),
                Field::text(
                    FieldId::RootName,
                    root.map(|root| root.fields().name.as_str())
                        .unwrap_or_default(),
                    false,
                ),
                Field::text(
                    FieldId::RootPath,
                    root.map(|root| root.fields().path.as_str())
                        .unwrap_or_default(),
                    false,
                ),
            ],
        );
        form.original_file_root = root.cloned();
        form
    }

    pub(crate) fn commit_file_root(
        &self,
        next: &mut Configuration,
        id: FileRootId,
        existing: bool,
    ) -> Result<()> {
        ensure!(
            text(&self.fields, FieldId::RootId)?.parse::<FileRootId>()? == id,
            INVALID
        );
        let root = FileRootConfiguration::new(FileRootFields {
            root_id: id,
            name: raw_text(&self.fields, FieldId::RootName)?.into(),
            path: raw_text(&self.fields, FieldId::RootPath)?.into(),
        })?;
        if existing {
            let current = next
                .file_roots
                .iter_mut()
                .find(|root| root.fields().root_id == id)
                .context(INVALID)?;
            ensure!(Some(&*current) == self.original_file_root.as_ref(), INVALID);
            *current = root;
        } else {
            ensure!(
                !next
                    .file_roots
                    .iter()
                    .any(|root| root.fields().root_id == id),
                INVALID
            );
            next.file_roots.push(root);
        }
        Ok(())
    }

    pub(crate) fn removal(&self) -> Option<Action> {
        match self.kind {
            FormKind::Export {
                peer,
                original: Some(origin),
            } => Some(Action::RemoveExport(peer, origin)),
            FormKind::FileRoot { existing: true, .. } => {
                self.original_file_root.clone().map(Action::RemoveFileRoot)
            }
            _ => None,
        }
    }
    pub(crate) fn removal_label(&self) -> &'static str {
        if matches!(self.kind, FormKind::FileRoot { .. }) {
            REMOVE_FILE_ROOT
        } else {
            REMOVE
        }
    }
}
