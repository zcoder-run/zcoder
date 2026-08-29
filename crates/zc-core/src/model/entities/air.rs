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
				let sql =
					"UPDATE run SET air_idx_seq = air_idx_seq + 1, mtime = ?2 WHERE id = ?1 RETURNING air_idx_seq";
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

	pub async fn list_for_run(mm: &ModelManager, run_id: Id) -> Result<Vec<Air>> {
		let filter = vec![SqliteField::new("run_id", run_id)];
		support::list::<Self, _>(mm, None, Some(filter.into())).await
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

	#[tokio::test]
	async fn test_model_air_bmc_list_for_run() -> Result<()> {
		// -- Setup & Fixtures
		let mm = get_model_manager()?;
		let run1_id = RunBmc::create(
			mm,
			RunForCreate {
				prompt: Some("run 1".to_string()),
				answer: None,
			},
		)
		.await?;
		let run2_id = RunBmc::create(
			mm,
			RunForCreate {
				prompt: Some("run 2".to_string()),
				answer: None,
			},
		)
		.await?;

		AirBmc::create_next(mm, run1_id, air_for_create(run1_id)).await?;
		AirBmc::create_next(mm, run1_id, air_for_create(run1_id)).await?;
		AirBmc::create_next(mm, run2_id, air_for_create(run2_id)).await?;

		// -- Exec
		let list1 = AirBmc::list_for_run(mm, run1_id).await?;
		let list2 = AirBmc::list_for_run(mm, run2_id).await?;

		// -- Check
		assert_eq!(list1.len(), 2);
		assert!(list1.iter().all(|a| a.run_id == run1_id));
		assert_eq!(list2.len(), 1);
		assert!(list2.iter().all(|a| a.run_id == run2_id));

		Ok(())
	}

	#[tokio::test]
	async fn test_model_air_bmc_model_event_on_create_and_update() -> Result<()> {
		// -- Setup & Fixtures
		let mm = get_model_manager()?;
		let mut bus_rx = get_model_bus().subscribe();

		let run_c = RunForCreate {
			prompt: Some("event test prompt".to_string()),
			answer: None,
		};
		let run_id = RunBmc::create(mm, run_c).await?;

		// -- Exec: Create Air
		let air_c = air_for_create(run_id);
		let air_id = AirBmc::create_next(mm, run_id, air_c).await?;

		// -- Check: Create Event
		let event = loop {
			let evt = bus_rx.recv().await?;
			if evt.entity == EntityType::Aixc && evt.id == Some(air_id) {
				break evt;
			}
		};
		assert_eq!(event.entity, EntityType::Aixc);
		assert_eq!(event.action, EntityAction::Created);
		assert_eq!(event.id, Some(air_id));
		assert_eq!(event.rel_ids.run_id, Some(run_id));

		// -- Exec: Update Air
		let update = AirForUpdate {
			end_state: Some(AirEndState::Success.to_string()),
			..Default::default()
		};
		let _ = AirBmc::update(mm, air_id, update).await?;

		// -- Check: Update Event
		let event = loop {
			let evt = bus_rx.recv().await?;
			if evt.entity == EntityType::Aixc && evt.id == Some(air_id) && evt.action == EntityAction::Updated {
				break evt;
			}
		};
		assert_eq!(event.entity, EntityType::Aixc);
		assert_eq!(event.action, EntityAction::Updated);
		assert_eq!(event.id, Some(air_id));

		Ok(())
	}

	#[tokio::test]
	async fn test_model_air_bmc_full_lifecycle_and_metrics() -> Result<()> {
		// -- Setup & Fixtures
		let mm = get_model_manager()?;
		let run_c = RunForCreate {
			prompt: Some("full lifecycle test".to_string()),
			answer: None,
		};
		let run_id = RunBmc::create(mm, run_c).await?;

		// -- Exec: Prep & Create Air
		let start = EpochUs::now();
		let chat_req = genai::chat::ChatRequest::from_messages(vec![genai::chat::ChatMessage::user("count to three")]);
		let air_c = crate::exec::prep_air_for_create(run_id, Some("test-model"), &chat_req, start, Some("step-label"));
		let air_id = AirBmc::create_next(mm, run_id, air_c).await?;

		// -- Check: Initial Air State
		let air = AirBmc::get(mm, air_id).await?;
		assert_eq!(air.run_id, run_id);
		assert_eq!(air.idx, 1);
		assert_eq!(air.model_ov.as_deref(), Some("test-model"));
		assert_eq!(air.label.as_deref(), Some("step-label"));
		assert_eq!(air.start, Some(start));
		assert!(air.prompt_json.ok_or("should have prompt_json")?.contains("count to three"));

		// -- Exec: Update with Success
		let model_iden = genai::ModelIden::from((genai::adapter::AdapterKind::OpenAI, "gpt-4o-mini"));
		let usage = genai::chat::Usage {
			prompt_tokens: Some(15),
			completion_tokens: Some(25),
			total_tokens: Some(40),
			prompt_tokens_details: Some(genai::chat::PromptTokensDetails {
				cached_tokens: Some(5),
				audio_tokens: None,
				cache_creation_tokens: None,
				cache_creation_details: None,
			}),
			completion_tokens_details: Some(genai::chat::CompletionTokensDetails {
				reasoning_tokens: Some(7),
				accepted_prediction_tokens: None,
				rejected_prediction_tokens: None,
				audio_tokens: None,
			}),
		};
		let res = genai::chat::ChatResponse {
			content: genai::chat::MessageContent::from("one two three"),
			reasoning_content: None,
			usage,
			model_iden: model_iden.clone(),
			provider_model_iden: model_iden,
			stop_reason: None,
			captured_raw_body: None,
			response_id: None,
		};
		let ai_start = EpochUs::now();
		let ai_end = EpochUs::now();
		let end = EpochUs::now();

		let update = crate::exec::prep_air_for_success(&res, Some(ai_start), Some(ai_end), Some(end));
		AirBmc::update(mm, air_id, update).await?;

		// -- Check: Updated Air State
		let air = AirBmc::get(mm, air_id).await?;
		assert_eq!(air.model_upstream.as_deref(), Some("gpt-4o-mini"));
		assert_eq!(air.token_in, Some(15));
		assert_eq!(air.token_out, Some(25));
		assert_eq!(air.token_reason, Some(7));
		assert_eq!(air.token_cache_hit, Some(5));
		assert_eq!(air.end_state.as_deref(), Some("success"));
		assert_eq!(air.ai_start, Some(ai_start));
		assert_eq!(air.ai_end, Some(ai_end));
		assert_eq!(air.end, Some(end));
		assert!(air.answer_json.ok_or("should have answer_json")?.contains("one two three"));

		// -- Exec & Check: Error Case
		let air_err_c = air_for_create(run_id);
		let err_air_id = AirBmc::create_next(mm, run_id, air_err_c).await?;
		let err_end = EpochUs::now();
		let err_update = crate::exec::prep_air_for_error("connection refused", err_end);
		AirBmc::update(mm, err_air_id, err_update).await?;

		let err_air = AirBmc::get(mm, err_air_id).await?;
		assert_eq!(err_air.idx, 2);
		assert_eq!(err_air.error.as_deref(), Some("connection refused"));
		assert_eq!(err_air.end_state.as_deref(), Some("error"));
		assert_eq!(err_air.end, Some(err_end));

		Ok(())
	}
}

// endregion: --- Tests
