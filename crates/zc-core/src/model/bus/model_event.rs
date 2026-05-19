use crate::model::{EntityType, Id, RelIds};
use derive_more::Deref;
use std::sync::Arc;

// region:    --- Types

#[derive(Clone, Deref)]
pub struct ModelEvent(Arc<ModelEventData>);

impl ModelEvent {
	pub fn new(entity: EntityType, action: EntityAction, id: Option<Id>, rel_ids: RelIds) -> Self {
		Self(Arc::new(ModelEventData {
			entity,
			action,
			id,
			rel_ids,
		}))
	}
}

#[allow(unused)]
#[derive(Debug, PartialEq, Eq)]
pub struct ModelEventData {
	pub entity: EntityType,
	pub action: EntityAction,
	pub id: Option<Id>,
	pub rel_ids: RelIds,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityAction {
	Created,
	Updated,
	Deleted,
}

// endregion: --- Types
