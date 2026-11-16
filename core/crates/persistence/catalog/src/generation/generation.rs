use super::constants::{CONTROLLER_GENERATION_EXHAUSTED, CONTROLLER_GENERATION_KEY};
use super::*;
use rusqlite::TransactionBehavior;

const READ_GENERATION_SQL: &str = "SELECT value FROM metadata WHERE key = ?1";
const WRITE_GENERATION_SQL: &str = "UPDATE metadata SET value = ?1 WHERE key = ?2";

impl Catalog {
    pub fn bump_controller_generation(&mut self) -> Result<u64> {
        self.claim_controller_generation(0)
    }

    pub fn claim_controller_generation(&mut self, floor: u64) -> Result<u64> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let effective = read_generation(&transaction)?.max(floor);
        let next = effective.checked_add(1);
        write_generation(&transaction, next.unwrap_or(effective))?;
        transaction.commit()?;
        next.context(CONTROLLER_GENERATION_EXHAUSTED)
    }

    pub fn controller_generation(&self) -> Result<u64> {
        read_generation(&self.connection)
    }

    pub fn restore_controller_generation_floor(&mut self, floor: u64) -> Result<()> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        if read_generation(&transaction)? < floor {
            write_generation(&transaction, floor)?;
        }
        transaction.commit()?;
        Ok(())
    }
}

pub(super) fn read_generation(connection: &Connection) -> Result<u64> {
    let encoded: String = super::rebuild_reason::schema_result(connection.query_row(
        READ_GENERATION_SQL,
        [CONTROLLER_GENERATION_KEY],
        |row| row.get(constants::METADATA_VALUE_COLUMN),
    ))?;
    let generation = encoded.parse::<u64>().map_err(|_| {
        super::rebuild_reason::content("controller generation is not an unsigned decimal")
    })?;
    if encoded != generation.to_string() {
        return Err(super::rebuild_reason::content(
            "controller generation is not canonical",
        ));
    }
    Ok(generation)
}

fn write_generation(connection: &Connection, generation: u64) -> Result<()> {
    let updated = connection.execute(
        WRITE_GENERATION_SQL,
        params![generation.to_string(), CONTROLLER_GENERATION_KEY],
    )?;
    if updated != 1 {
        return Err(super::rebuild_reason::content(
            "controller generation metadata is missing",
        ));
    }
    Ok(())
}
