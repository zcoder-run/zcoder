use crate::model::support::{self, DbBmc};
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
	pub end: Option<EpochUs>,
	pub end_state: Option<String>,
	pub total_cost: Option<f64>,
	pub air_idx_seq: i64,
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
	pub end: Option<EpochUs>,
	pub end_state: Option<String>,
	pub total_cost: Option<f64>,
}

/// End state for a Run execution.
#[derive(Debug, Clone, PartialEq, Eq, derive_more::Display)]
pub enum RunEndState {
	#[display("success")]
	Success,
	#[display("error")]
	Error,
	#[display("cancelled")]
	Cancelled,
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
	pub async fn create(mm: &ModelManager, run_c: RunForCreate) -> Result<Id> {
		let fields = run_c.sqlite_not_none_fields();
		support::create::<Self>(mm, fields).await
	}

	#[allow(unused)]
	pub async fn update(mm: &ModelManager, id: Id, run_u: RunForUpdate) -> Result<usize> {
		let fields = run_u.sqlite_not_none_fields();
		support::update::<Self>(mm, id, fields).await
	}

	#[allow(unused)]
	pub async fn get(mm: &ModelManager, id: Id) -> Result<Run> {
		support::get::<Self, _>(mm, id).await
	}

	pub async fn list(mm: &ModelManager, list_options: Option<ListOptions>) -> Result<Vec<Run>> {
		support::list::<Self, _>(mm, list_options, None).await
	}

	pub async fn recompute_total_cost(mm: &ModelManager, run_id: Id) -> Result<f64> {
		let airs = super::AirBmc::list_for_run(mm, run_id).await?;
		let total_cost: f64 = airs.iter().filter_map(|a| a.cost).sum();
		let run_u = RunForUpdate {
			total_cost: Some(total_cost),
			..Default::default()
		};
		Self::update(mm, run_id, run_u).await?;
		Ok(total_cost)
	}
}

// endregion: --- Bmc

// region:    --- Tests

#[cfg(test)]
mod tests {
	use super::*;
	use crate::model::model_manager::get_model_manager;
	type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>; // For tests.

	#[tokio::test]
	async fn test_model_run_bmc_create() -> Result<()> {
		// -- Fixture
		let mm = get_model_manager()?;
		let run_c = RunForCreate {
			prompt: Some("Why is shy red?".to_string()),
			answer: Some("Because not happy.".to_string()),
		};

		// -- Exec
		let id = RunBmc::create(mm, run_c).await?;

		// -- Check
		let run = RunBmc::get(mm, id).await?;
		assert_eq!(run.prompt.as_deref(), Some("Why is shy red?"));

		Ok(())
	}

	#[tokio::test]
	async fn test_model_run_bmc_update_end_state() -> Result<()> {
		// -- Setup & Fixtures
		let mm = get_model_manager()?;
		let run_c = RunForCreate {
			prompt: Some("compute task".to_string()),
			answer: None,
		};
		let id = RunBmc::create(mm, run_c).await?;

		// -- Exec
		let end_time = EpochUs::now();
		let run_u = RunForUpdate {
			answer: Some("task finished".to_string()),
			end: Some(end_time),
			end_state: Some(RunEndState::Success.to_string()),
			..Default::default()
		};
		let count = RunBmc::update(mm, id, run_u).await?;

		// -- Check
		assert_eq!(count, 1);
		let run = RunBmc::get(mm, id).await?;
		assert_eq!(run.answer.as_deref(), Some("task finished"));
		assert_eq!(run.end, Some(end_time));
		assert_eq!(run.end_state.as_deref(), Some("success"));

		Ok(())
	}

	#[tokio::test]
	async fn test_model_run_bmc_recompute_total_cost() -> Result<()> {
		// -- Setup & Fixtures
		let mm = get_model_manager()?;
		let run_c = RunForCreate {
			prompt: Some("cost aggregate test".to_string()),
			answer: None,
		};
		let run_id = RunBmc::create(mm, run_c).await?;

		let air1 = crate::model::AirForCreate {
			run_id,
			cost: Some(0.0125),
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
			error: None,
			end_state: None,
			start: None,
			ai_start: None,
			ai_end: None,
			end: None,
		};
		let air2 = crate::model::AirForCreate {
			run_id,
			cost: Some(0.0375),
			..air1.clone()
		};
		crate::model::AirBmc::create_next(mm, run_id, air1).await?;
		crate::model::AirBmc::create_next(mm, run_id, air2).await?;

		// -- Exec
		let total = RunBmc::recompute_total_cost(mm, run_id).await?;

		// -- Check
		assert!((total - 0.05).abs() < 1e-6);
		let run = RunBmc::get(mm, run_id).await?;
		let run_cost = run.total_cost.ok_or("should have total_cost")?;
		assert!((run_cost - 0.05).abs() < 1e-6);

		Ok(())
	}
}

// endregion: --- Tests
