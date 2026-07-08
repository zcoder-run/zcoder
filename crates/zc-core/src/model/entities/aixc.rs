// region:    --- Modules

use crate::model::base::prep_fields::prep_fields_for_create;
use crate::model::base::{self, DbBmc};
use crate::model::{EntityAction, EntityType, EpochUs, Id, ModelEvent, ModelManager, RelIds, Result, get_model_bus};
use modql::SqliteFromRow;
use modql::field::{Fields, HasSqliteFields, SqliteField};
use modql::filter::ListOptions;

// endregion: --- Modules

// region:    --- Types

#[derive(Debug, Clone, Fields, SqliteFromRow)]
pub struct Aixc {
	pub id: Id,

	pub run_id: Id,
	pub idx: i64,

	pub label: Option<String>,

	pub ctime: EpochUs,
	pub mtime: EpochUs,

	pub start: Option<EpochUs>,
	pub ai_start: Option<EpochUs>,
	pub ai_end: Option<EpochUs>,
	pub end: Option<EpochUs>,

	pub model_ov: Option<String>,
	pub model_upstream: Option<String>,
	pub prompt_json: Option<String>,
	pub answer_json: Option<String>,
	pub usage_json: Option<String>,

	pub token_in: Option<i64>,
	pub token_out: Option<i64>,
	pub token_reason: Option<i64>,
	pub token_cache_hit: Option<i64>,
	pub token_cache_write: Option<i64>,

	pub cost: Option<f64>,

	pub error: Option<String>,
	pub end_state: Option<String>,
}

#[derive(Debug, Clone, Fields, SqliteFromRow)]
pub struct AixcForCreate {
	pub run_id: Id,

	pub label: Option<String>,

	pub model_ov: Option<String>,
	pub model_upstream: Option<String>,
	pub prompt_json: Option<String>,
	pub answer_json: Option<String>,
	pub usage_json: Option<String>,

	pub token_in: Option<i64>,
	pub token_out: Option<i64>,
	pub token_reason: Option<i64>,
	pub token_cache_hit: Option<i64>,
	pub token_cache_write: Option<i64>,

	pub cost: Option<f64>,

	pub error: Option<String>,
	pub end_state: Option<String>,

	pub start: Option<EpochUs>,
	pub ai_start: Option<EpochUs>,
	pub ai_end: Option<EpochUs>,
	pub end: Option<EpochUs>,
}

#[derive(Debug, Default, Clone, Fields, SqliteFromRow)]
pub struct AixcForUpdate {
	pub label: Option<String>,

	pub model_ov: Option<String>,
	pub model_upstream: Option<String>,
	pub prompt_json: Option<String>,
	pub answer_json: Option<String>,
	pub usage_json: Option<String>,

	pub token_in: Option<i64>,
	pub token_out: Option<i64>,
	pub token_reason: Option<i64>,
	pub token_cache_hit: Option<i64>,
	pub token_cache_write: Option<i64>,

	pub cost: Option<f64>,

	pub error: Option<String>,
	pub end_state: Option<String>,

	pub start: Option<EpochUs>,
	pub ai_start: Option<EpochUs>,
	pub ai_end: Option<EpochUs>,
	pub end: Option<EpochUs>,
}

/// End state for an AI execution.
#[derive(Debug, Clone, PartialEq, Eq, derive_more::Display)]
pub enum AixcEndState {
	#[display("success")]
	Success,
	#[display("error")]
	Error,
	#[display("cancelled")]
	Cancelled,
}

// endregion: --- Types

// region:    --- Bmc

pub struct AixcBmc;

impl DbBmc for AixcBmc {
	const TABLE: &'static str = "aixc";
	const ENTITY_TYPE: EntityType = EntityType::Aixc;
}

/// Basic CRUD
impl AixcBmc {
	pub fn create(mm: &ModelManager, aixc_c: AixcForCreate) -> Result<Id> {
		let run_id = aixc_c.run_id;
		Self::create_next(mm, run_id, aixc_c)
	}

	/// Atomically increments the Run's `aixc_idx_seq`, then creates a new Aixc record
	/// with that sequence number as `idx`.
	pub fn create_next(mm: &ModelManager, run_id: Id, aixc_c: AixcForCreate) -> Result<Id> {
		let db = mm.db();
		let rel_ids = RelIds { run_id: Some(run_id) };

		let id = db.exec_in_tx(|tx_db| {
			// Atomically increment aixc_idx_seq on the Run record
			let sql = "UPDATE run SET aixc_idx_seq = aixc_idx_seq + 1, mtime = ?2 WHERE id = ?1 RETURNING aixc_idx_seq";
			let now = zc_common::time::now_micro();
			let new_idx: i64 = tx_db.exec_returning_as(sql, (run_id, now))?;

			// Build fields for the Aixc record (includes run_id from aixc_c)
			let mut fields = aixc_c.sqlite_not_none_fields();
			fields.push(SqliteField::new("idx", new_idx));
			prep_fields_for_create::<Self>(&mut fields);

			let sql = format!(
				"INSERT INTO {} ({}) VALUES ({}) RETURNING id",
				Self::TABLE,
				fields.sql_columns(),
				fields.sql_placeholders()
			);

			let values = fields.values_as_dyn_to_sql_vec();
			let id: Id = tx_db.exec_returning_as(&sql, &*values)?;

			Ok(id)
		})?;

		// Publish Model Event
		get_model_bus().publish(ModelEvent::new(
			Self::ENTITY_TYPE,
			EntityAction::Created,
			Some(id),
			rel_ids,
		));

		Ok(id)
	}

	#[allow(unused)]
	pub fn update(mm: &ModelManager, id: Id, aixc_u: AixcForUpdate) -> Result<usize> {
		let fields = aixc_u.sqlite_not_none_fields();
		base::update::<Self>(mm, id, fields)
	}

	#[allow(unused)]
	pub fn get(mm: &ModelManager, id: Id) -> Result<Aixc> {
		base::get::<Self, _>(mm, id)
	}

	#[allow(unused)]
	pub fn list(mm: &ModelManager, list_options: Option<ListOptions>) -> Result<Vec<Aixc>> {
		base::list::<Self, _>(mm, list_options, None)
	}
}

// endregion: --- Bmc

// region:    --- Tests

#[cfg(test)]
mod tests {
	type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>; // For tests.

	use super::*;
	use crate::model::{RunBmc, RunForCreate, get_model_manager};

	// region:    --- Support

	fn aixc_for_create(run_id: Id) -> AixcForCreate {
		AixcForCreate {
			run_id,
			label: None,
			model_ov: None,
			model_upstream: None,
			prompt_json: None,
			answer_json: None,
			usage_json: None,
			token_in: None,
			token_out: None,
			token_reason: None,
			token_cache_hit: None,
			token_cache_write: None,
			cost: None,
			error: None,
			end_state: None,
			start: None,
			ai_start: None,
			ai_end: None,
			end: None,
		}
	}

	// endregion: --- Support

	#[test]
	fn test_model_aixc_bmc_create_next() -> Result<()> {
		// -- Setup & Fixtures
		let mm = get_model_manager()?;
		let run_c = RunForCreate {
			prompt: Some("test prompt".to_string()),
			answer: Some("test answer".to_string()),
		};
		let run_id = RunBmc::create(mm, run_c)?;

		let mut aixc_c = aixc_for_create(run_id);
		aixc_c.label = Some("first call".to_string());
		aixc_c.model_ov = Some("gpt-4".to_string());

		// -- Exec
		let aixc_id = AixcBmc::create_next(mm, run_id, aixc_c)?;

		// -- Check
		let aixc = AixcBmc::get(mm, aixc_id)?;
		assert_eq!(aixc.run_id, run_id);
		assert_eq!(aixc.idx, 1);
		assert_eq!(aixc.label.as_deref(), Some("first call"));
		assert_eq!(aixc.model_ov.as_deref(), Some("gpt-4"));

		let run = RunBmc::get(mm, run_id)?;
		assert_eq!(run.aixc_idx_seq, 1);

		Ok(())
	}

	#[test]
	fn test_model_aixc_bmc_create_multiple_nexts() -> Result<()> {
		// -- Setup & Fixtures
		let mm = get_model_manager()?;
		let run_c = RunForCreate {
			prompt: Some("multi".to_string()),
			answer: None,
		};
		let run_id = RunBmc::create(mm, run_c)?;

		// -- Exec & Check
		let id1 = AixcBmc::create_next(mm, run_id, {
			let mut c = aixc_for_create(run_id);
			c.label = Some("first".to_string());
			c
		})?;
		let id2 = AixcBmc::create_next(mm, run_id, {
			let mut c = aixc_for_create(run_id);
			c.label = Some("second".to_string());
			c
		})?;
		let id3 = AixcBmc::create_next(mm, run_id, {
			let mut c = aixc_for_create(run_id);
			c.label = Some("third".to_string());
			c
		})?;

		let a1 = AixcBmc::get(mm, id1)?;
		let a2 = AixcBmc::get(mm, id2)?;
		let a3 = AixcBmc::get(mm, id3)?;

		assert_eq!(a1.idx, 1);
		assert_eq!(a2.idx, 2);
		assert_eq!(a3.idx, 3);

		let run = RunBmc::get(mm, run_id)?;
		assert_eq!(run.aixc_idx_seq, 3);

		Ok(())
	}

	#[test]
	fn test_model_aixc_bmc_update() -> Result<()> {
		// -- Setup & Fixtures
		let mm = get_model_manager()?;
		let run_c = RunForCreate {
			prompt: Some("update test".to_string()),
			answer: None,
		};
		let run_id = RunBmc::create(mm, run_c)?;

		let aixc_id = AixcBmc::create_next(mm, run_id, aixc_for_create(run_id))?;

		let update = AixcForUpdate {
			label: Some("updated label".to_string()),
			model_ov: Some("claude-3".to_string()),
			..Default::default()
		};

		// -- Exec
		let count = AixcBmc::update(mm, aixc_id, update)?;

		// -- Check
		assert_eq!(count, 1);
		let aixc = AixcBmc::get(mm, aixc_id)?;
		assert_eq!(aixc.label.as_deref(), Some("updated label"));
		assert_eq!(aixc.model_ov.as_deref(), Some("claude-3"));

		Ok(())
	}

	#[test]
	fn test_model_aixc_bmc_list() -> Result<()> {
		// -- Setup & Fixtures
		let mm = get_model_manager()?;
		let run_c = RunForCreate {
			prompt: Some("list test".to_string()),
			answer: None,
		};
		let run_id = RunBmc::create(mm, run_c)?;
		AixcBmc::create_next(mm, run_id, aixc_for_create(run_id))?;
		AixcBmc::create_next(mm, run_id, aixc_for_create(run_id))?;

		// -- Exec
		let list = AixcBmc::list(mm, None)?;

		// -- Check
		assert!(list.len() >= 2);

		Ok(())
	}
}

// endregion: --- Tests
