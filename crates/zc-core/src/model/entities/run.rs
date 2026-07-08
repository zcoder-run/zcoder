use crate::model::base::{self, DbBmc};
use crate::model::{EntityType, EpochUs, Id, ModelManager, Result};
use modql::SqliteFromRow;
use modql::field::{Fields, HasSqliteFields};
use modql::filter::ListOptions;

// region:    --- Types

#[derive(Debug, Clone, Fields, SqliteFromRow)]
pub struct Run {
	pub id: Id,

	pub ctime: EpochUs,
	pub mtime: EpochUs,

	pub prompt: Option<String>,
	pub answer: Option<String>,
	pub error: Option<String>,
	pub aixc_idx_seq: i64,
}

#[derive(Debug, Clone, Fields, SqliteFromRow)]
pub struct RunForCreate {
	pub prompt: Option<String>,
	pub answer: Option<String>,
}

#[derive(Debug, Default, Clone, Fields, SqliteFromRow)]
pub struct RunForUpdate {
	pub prompt: Option<String>,
	pub answer: Option<String>,
	pub error: Option<String>,
}

// endregion: --- Types

// region:    --- Bmc

pub struct RunBmc;

impl DbBmc for RunBmc {
	const TABLE: &'static str = "run";
	const ENTITY_TYPE: EntityType = EntityType::Run;
}

/// Basic CRUD
impl RunBmc {
	pub fn create(mm: &ModelManager, run_c: RunForCreate) -> Result<Id> {
		let fields = run_c.sqlite_not_none_fields();
		base::create::<Self>(mm, fields)
	}

	#[allow(unused)]
	pub fn update(mm: &ModelManager, id: Id, run_u: RunForUpdate) -> Result<usize> {
		let fields = run_u.sqlite_not_none_fields();
		base::update::<Self>(mm, id, fields)
	}

	#[allow(unused)]
	pub fn get(mm: &ModelManager, id: Id) -> Result<Run> {
		base::get::<Self, _>(mm, id)
	}

	pub fn list(mm: &ModelManager, list_options: Option<ListOptions>) -> Result<Vec<Run>> {
		base::list::<Self, _>(mm, list_options, None)
	}
}

// endregion: --- Bmc

// region:    --- Tests

#[cfg(test)]
mod tests {
	use super::*;
	use crate::model::model_manager::get_model_manager;
	type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>; // For tests.

	#[test]
	fn test_model_run_bmc_create() -> Result<()> {
		// -- Fixture
		let mm = get_model_manager()?;
		let run_c = RunForCreate {
			prompt: Some("Why is shy red?".to_string()),
			answer: Some("Because not happy.".to_string()),
		};

		// -- Exec
		let id = RunBmc::create(mm, run_c)?;

		// -- Check
		let run = RunBmc::get(mm, id)?;
		assert_eq!(run.prompt.as_deref(), Some("Why is shy red?"));

		Ok(())
	}
}

// endregion: --- Tests
