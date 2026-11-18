use anyhow::Result;

pub fn rewrite_spec_boot(
    spec: &mut super::CustodySpec,
    current_boot_id: &str,
    previous_boot_id: &str,
) -> Result<()> {
    super::reboot::rewrite_spec_boot_for_test(spec, current_boot_id, previous_boot_id)
}

pub fn rewrite_ledger_boot(
    ledger: &mut super::CustodyLedger,
    current_boot_id: &str,
    previous_boot_id: &str,
) -> Result<()> {
    for record in &mut ledger.records {
        rewrite_spec_boot(&mut record.spec, current_boot_id, previous_boot_id)?;
    }
    Ok(())
}
