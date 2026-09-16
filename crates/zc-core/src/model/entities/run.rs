use crate::model::{EpochUs, Id};
use modql::SqliteFromRow;
use modql::field::Fields;
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

/// List options for querying runs.
pub type ListRunOptions = ListOptions;

// endregion: --- Types
