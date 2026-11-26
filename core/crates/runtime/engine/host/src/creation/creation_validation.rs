use super::*;

impl Host {
    pub(crate) async fn existing_bound_manifest(
        &self,
        binding: &CreationBinding,
    ) -> std::result::Result<Option<TerminalManifest>, ErrorBody> {
        self.existing_bound_manifest_with_deadline(binding, None)
            .await
    }

    pub(crate) async fn existing_bound_manifest_before(
        &self,
        binding: &CreationBinding,
        deadline: HostOperationDeadline,
    ) -> std::result::Result<Option<TerminalManifest>, ErrorBody> {
        self.existing_bound_manifest_with_deadline(binding, Some(deadline))
            .await
    }

    pub(crate) async fn existing_bound_manifest_with_deadline(
        &self,
        binding: &CreationBinding,
        deadline: Option<HostOperationDeadline>,
    ) -> std::result::Result<Option<TerminalManifest>, ErrorBody> {
        let catalog_manifest = self
            .catalog_get(binding.result.terminal_id)
            .await
            .map_err(internal)?;
        let durable_manifest: Option<TerminalManifest> = read_creation_evidence(
            &self.paths.manifest(binding.result.terminal_id),
            "creation session manifest",
        )
        .map_err(internal)?;
        match (catalog_manifest, durable_manifest) {
            (None, None) => Ok(None),
            (Some(catalog), Some(durable)) => {
                if !manifest_matches_binding(&catalog, binding)
                    || !manifest_matches_binding(&durable, binding)
                    || !same_manifest_identity(&catalog, &durable)
                {
                    return Err(ErrorBody::new(
                        crate::protocol::error_code::IDENTITY_MISMATCH,
                        "reserved creation does not match its durable session",
                        false,
                    ));
                }
                if same_manifest(&catalog, &durable) {
                    Ok(Some(catalog))
                } else {
                    self.upsert_creation_catalog(&durable, deadline).await?;
                    Ok(Some(durable))
                }
            }
            (Some(catalog), None) if manifest_matches_binding(&catalog, binding) => {
                self.persist_creation_manifest(&catalog, deadline)?;
                Ok(Some(catalog))
            }
            (None, Some(durable)) if manifest_matches_binding(&durable, binding) => {
                self.insert_creation_catalog(&durable, deadline).await?;
                Ok(Some(durable))
            }
            _ => Err(ErrorBody::new(
                crate::protocol::error_code::IDENTITY_MISMATCH,
                "reserved creation has mismatched durable session evidence",
                false,
            )),
        }
    }

    pub(crate) fn persist_creation_manifest(
        &self,
        manifest: &TerminalManifest,
        deadline: Option<HostOperationDeadline>,
    ) -> std::result::Result<(), ErrorBody> {
        let path = self.paths.manifest(manifest.terminal_id);
        match deadline {
            Some(deadline) => self.write_creation_json_before(&path, manifest, deadline),
            None => self.write_namespace_json(&path, manifest).map_err(internal),
        }
    }

    pub(crate) async fn insert_creation_catalog(
        &self,
        manifest: &TerminalManifest,
        deadline: Option<HostOperationDeadline>,
    ) -> std::result::Result<(), ErrorBody> {
        match deadline {
            Some(deadline) => self
                .catalog_insert_before(manifest, deadline)
                .await
                .map_err(creation_publication_error),
            None => self.catalog_insert(manifest).await.map_err(internal),
        }
    }

    pub(crate) async fn upsert_creation_catalog(
        &self,
        manifest: &TerminalManifest,
        deadline: Option<HostOperationDeadline>,
    ) -> std::result::Result<(), ErrorBody> {
        match deadline {
            Some(deadline) => self
                .catalog_upsert_before(manifest, deadline)
                .await
                .map_err(creation_publication_error),
            None => self.catalog_upsert(manifest).await.map_err(internal),
        }
    }
}

pub(in crate::host) fn manifest_matches_binding(manifest: &TerminalManifest, binding: &CreationBinding) -> bool {
    let shell = shell_command(&binding.spec.cwd);
    let expected_name = binding
        .spec
        .name
        .clone()
        .unwrap_or_else(|| command_basename(shell.executable.to_string_lossy().as_ref()));
    manifest.schema_version == SESSION_MANIFEST_SCHEMA_VERSION
        && manifest.terminal_id == binding.result.terminal_id
        && manifest.name == expected_name
        && manifest.command == shell.executable.display().to_string()
        && manifest.args == shell.args
        && manifest.cwd == binding.spec.cwd
        && manifest.cols
            == normalize_terminal_columns(binding.spec.cols.unwrap_or(DEFAULT_COLS))
        && manifest.rows
            == normalize_terminal_rows(binding.spec.rows.unwrap_or(DEFAULT_ROWS))
        && !manifest.archived
}

pub(crate) fn same_manifest(left: &TerminalManifest, right: &TerminalManifest) -> bool {
    left == right
}

pub(crate) fn same_manifest_identity(left: &TerminalManifest, right: &TerminalManifest) -> bool {
    left.schema_version == right.schema_version
        && left.terminal_id == right.terminal_id
        && left.name == right.name
        && left.command == right.command
        && left.args == right.args
        && left.cwd == right.cwd
        && left.cols == right.cols
        && left.rows == right.rows
        && left.created_at_ms == right.created_at_ms
        && left.archived == right.archived
}

include!("creation_runtime_evidence.rs");
