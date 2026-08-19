// region:    --- Modules

use crate::model::support::prep_fields::prep_fields_for_create;
use crate::model::support::{self, DbBmc};
use crate::model::{EntityAction, EntityType, EpochUs, Id, ModelEvent, ModelManager, RelIds, Result, get_model_bus};
use modql::SqliteFromRow;
use modql::field::{Fields, HasSqliteFields, SqliteField};
use modql::filter::ListOptions;

// endregion: --- Modules

// region:    --- Types

/// AI Request
#[derive(Debug, Clone, Fields, SqliteFromRow)]
pub struct Air {
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
pub struct AirForCreate {
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
pub struct AirForUpdate {
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
pub enum AirEndState {
	#[display("success")]
	Success,
	#[display("error")]
	Error,
	#[display("cancelled")]
	Cancelled,
}

// endregion: --- Types

// region:    --- Bmc

pub struct AirBmc;

impl DbBmc for AirBmc {
	const TABLE: &'static str = "aixc";
	const ENTITY_TYPE: EntityType = EntityType::Aixc;
}

/// Basic CRUD
impl AirBmc {
	pub async fn create(mm: &ModelManager, air_c: AirForCreate) -> Result<Id> {
		let run_id = air_c.run_id;
		Self::create_next(mm, run_id, air_c).await
	}

	/// Atomically increments the Run's `air_idx_seq`, then creates a new Aixc record
	/// with that sequence number as `idx`.
	pub async fn create_next(mm: &ModelManager, run_id: Id, air_c: AirForCreate) -> Result<Id> {
		let db = mm.db();
		let rel_ids = RelIds { run_id: Some(run_id) };

		let id = db
			.exec_in_tx(|tx_db| {
				// Atomically increment air_idx_seq on the Run record
				let sql = "UPDATE run SET air_idx_seq = air_idx_seq + 1, mtime = ?2 WHERE id = ?1 RETURNING air_idx_seq";
				let now = zc_common::time::now_micro();
				let new_idx: i64 = tx_db.exec_returning_as(sql, (run_id, now))?;

				// Build fields for the Aixc record (includes run_id from air_c)
				let mut fields = air_c.sqlite_not_none_fields();
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
			})
			.await?;

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
	pub async fn update(mm: &ModelManager, id: Id, air_u: AirForUpdate) -> Result<usize> {
		let fields = air_u.sqlite_not_none_fields();
		support::update::<Self>(mm, id, fields).await
	}

	#[allow(unused)]
	pub async fn get(mm: &ModelManager, id: Id) -> Result<Air> {
		support::get::<Self, _>(mm, id).await
	}

	#[allow(unused)]
	pub async fn list(mm: &ModelManager, list_options: Option<ListOptions>) -> Result<Vec<Air>> {
		support::list::<Self, _>(mm, list_options, None).await
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

	fn air_for_create(run_id: Id) -> AirForCreate {
		AirForCreate {
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

	#[tokio::test]
	async fn test_model_air_bmc_create_next() -> Result<()> {
		// -- Setup & Fixtures
		let mm = get_model_manager()?;
		let run_c = RunForCreate {
			prompt: Some("test prompt".to_string()),
			answer: Some("test answer".to_string()),
		};
		let run_id = RunBmc::create(mm, run_c).await?;

		let mut air_c = air_for_create(run_id);
		air_c.label = Some("first call".to_string());
		air_c.model_ov = Some("gpt-4".to_string());

		// -- Exec
		let air_id = AirBmc::create_next(mm, run_id, air_c).await?;

		// -- Check
		let aixc = AirBmc::get(mm, air_id).await?;
		assert_eq!(aixc.run_id, run_id);
		assert_eq!(aixc.idx, 1);
		assert_eq!(aixc.label.as_deref(), Some("first call"));
		assert_eq!(aixc.model_ov.as_deref(), Some("gpt-4"));

		let run = RunBmc::get(mm, run_id).await?;
		assert_eq!(run.air_idx_seq, 1);

		Ok(())
	}

	#[tokio::test]
	async fn test_model_air_bmc_create_multiple_nexts() -> Result<()> {
		// -- Setup & Fixtures
		let mm = get_model_manager()?;
		let run_c = RunForCreate {
			prompt: Some("multi".to_string()),
			answer: None,
		};
		let run_id = RunBmc::create(mm, run_c).await?;

		// -- Exec & Check
		let id1 = AirBmc::create_next(mm, run_id, {
			let mut c = air_for_create(run_id);
			c.label = Some("first".to_string());
			c
		})
		.await?;
		let id2 = AirBmc::create_next(mm, run_id, {
			let mut c = air_for_create(run_id);
			c.label = Some("second".to_string());
			c
		})
		.await?;
		let id3 = AirBmc::create_next(mm, run_id, {
			let mut c = air_for_create(run_id);
			c.label = Some("third".to_string());
			c
		})
		.await?;

		let a1 = AirBmc::get(mm, id1).await?;
		let a2 = AirBmc::get(mm, id2).await?;
		let a3 = AirBmc::get(mm, id3).await?;

		assert_eq!(a1.idx, 1);
		assert_eq!(a2.idx, 2);
		assert_eq!(a3.idx, 3);

		let run = RunBmc::get(mm, run_id).await?;
		assert_eq!(run.air_idx_seq, 3);

		Ok(())
	}

	#[tokio::test]
	async fn test_model_air_bmc_update() -> Result<()> {
		// -- Setup & Fixtures
		let mm = get_model_manager()?;
		let run_c = RunForCreate {
			prompt: Some("update test".to_string()),
			answer: None,
		};
		let run_id = RunBmc::create(mm, run_c).await?;

		let air_id = AirBmc::create_next(mm, run_id, air_for_create(run_id)).await?;

		let update = AirForUpdate {
			label: Some("updated label".to_string()),
			model_ov: Some("claude-3".to_string()),
			..Default::default()
		};

		// -- Exec
		let count = AirBmc::update(mm, air_id, update).await?;

		// -- Check
		assert_eq!(count, 1);
		let aixc = AirBmc::get(mm, air_id).await?;
		assert_eq!(aixc.label.as_deref(), Some("updated label"));
		assert_eq!(aixc.model_ov.as_deref(), Some("claude-3"));

		Ok(())
	}

	#[tokio::test]
	async fn test_model_air_bmc_list() -> Result<()> {
		// -- Setup & Fixtures
		let mm = get_model_manager()?;
		let run_c = RunForCreate {
			prompt: Some("list test".to_string()),
			answer: None,
		};
		let run_id = RunBmc::create(mm, run_c).await?;
		AirBmc::create_next(mm, run_id, air_for_create(run_id)).await?;
		AirBmc::create_next(mm, run_id, air_for_create(run_id)).await?;

		// -- Exec
		let list = AirBmc::list(mm, None).await?;

		// -- Check
		assert!(list.len() >= 2);

		Ok(())
	}
}

// endregion: --- Tests
