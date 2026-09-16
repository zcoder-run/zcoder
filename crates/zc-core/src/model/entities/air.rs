// region:    --- Modules

use crate::model::{EpochUs, Id};
use modql::SqliteFromRow;
use modql::field::Fields;
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

/// List options for querying AI requests.
pub type ListAirOptions = ListOptions;

// endregion: --- Types
