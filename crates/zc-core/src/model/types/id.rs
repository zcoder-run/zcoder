use crate::ScalarStruct;
use crate::model::{Error, Result};
use macro_rules_attribute as mra;
use uuid::Uuid;

// Simple wrapper for SQLite Ids
#[mra::derive(Debug, ScalarStruct!)]
pub struct Id(Uuid);

impl Id {
	pub fn as_uuid(&self) -> &Uuid {
		&self.0
	}

	pub fn into_uuid(self) -> Uuid {
		self.0
	}
}

// from &i64
impl From<&Uuid> for Id {
	fn from(val: &Uuid) -> Id {
		Id(*val)
	}
}

impl TryFrom<String> for Id {
	type Error = Error;
	fn try_from(val: String) -> Result<Id> {
		let uuid =
			Uuid::parse_str(&val).map_err(|err| format!("id should be a valid UUID, was '{val}'.\nCause: {err}"))?;
		Ok(Id(uuid))
	}
}

// region:    --- Tests

#[cfg(test)]
mod tests {
	use super::*;
	use rusqlite::{Connection, params};

	type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>; // For tests.

	#[test]
	fn test_sqlite_uuid_blob_roundtrip() -> Result<()> {
		let con = Connection::open_in_memory()?;
		let uuid = Uuid::now_v7();

		con.execute("CREATE TABLE item (id BLOB PRIMARY KEY) STRICT, WITHOUT ROWID", ())?;
		con.execute("INSERT INTO item (id) VALUES (?1)", params![&uuid])?;

		let stored_id: Id = con.query_row("SELECT id FROM item LIMIT 1", [], |row| row.get(0))?;
		let selected_id: Id = con.query_row("SELECT id FROM item WHERE id = ?1", params![&stored_id], |row| {
			row.get(0)
		})?;
		let stored_bytes: Vec<u8> = con.query_row("SELECT id FROM item LIMIT 1", [], |row| row.get(0))?;

		assert_eq!(stored_id.as_uuid(), &uuid);
		assert_eq!(selected_id.as_uuid(), &uuid);
		assert_eq!(stored_bytes.len(), 16);

		Ok(())
	}
}

// endregion: --- Tests
