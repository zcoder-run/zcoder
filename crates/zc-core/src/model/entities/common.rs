use crate::model::Id;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EntityType {
	Run,
	Aixc,
	Wks,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelIds {
	pub run_id: Option<Id>,
	pub wspace_id: Option<Id>,
}
