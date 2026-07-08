use crate::model::Id;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityType {
	Run,
	Aixc,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RelIds {
	pub run_id: Option<Id>,
}
