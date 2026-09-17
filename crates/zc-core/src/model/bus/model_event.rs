use crate::model::{EntityType, Id, RelIds};
use derive_more::Deref;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

// region:    --- Types

#[derive(Debug, Clone, Deref, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ModelChangeEvent(Arc<ModelEventData>);

impl ModelChangeEvent {
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelEventData {
	pub entity: EntityType,
	pub action: EntityAction,
	pub id: Option<Id>,
	pub rel_ids: RelIds,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntityAction {
	Created,
	Updated,
	Deleted,
}

// endregion: --- Types

// region:    --- Tests

#[cfg(test)]
mod tests {
	use super::*;

	type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>;

	#[test]
	fn test_model_change_event_serde_roundtrip() -> Result<()> {
		let event = ModelChangeEvent::new(
			EntityType::Run,
			EntityAction::Created,
			Some(Id::default()),
			RelIds::default(),
		);
		let json = serde_json::to_string(&event)?;
		let event_de: ModelChangeEvent = serde_json::from_str(&json)?;
		assert_eq!(event.entity, event_de.entity);
		assert_eq!(event.action, event_de.action);
		assert_eq!(event.id, event_de.id);
		assert_eq!(event.rel_ids, event_de.rel_ids);
		Ok(())
	}
}

// endregion: --- Tests
