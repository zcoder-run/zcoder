use crate::model::{EpochUs, Id};
use modql::SqliteFromRow;
use modql::field::Fields;
use modql::filter::ListOptions;
use serde::{Deserialize, Serialize};

// region:    --- Types

#[derive(Debug, Clone, Fields, SqliteFromRow, Serialize, Deserialize)]
pub struct Wks {
	pub id: Id,

	pub ctime: EpochUs,
	pub mtime: EpochUs,

	pub dir: String,
	pub label: Option<String>,
}

#[derive(Debug, Clone, Fields, SqliteFromRow, Serialize, Deserialize)]
pub struct WksForCreate {
	pub dir: String,
	pub label: Option<String>,
}

/// List options for querying workspaces.
pub type ListWksOptions = ListOptions;

// endregion: --- Types
