use crate::model::{EntityType, Id, ModelManager, Result, base};
use uuid::Uuid;

pub trait DbBmc: Sized {
	const TABLE: &'static str;
	const ENTITY_TYPE: EntityType;

	fn table_ref() -> &'static str {
		Self::TABLE
	}

	#[allow(unused)]
	fn get_uid(mm: &ModelManager, id: Id) -> Result<Uuid> {
		base::get_uid::<Self>(mm, id)
	}

	#[allow(unused)]
	fn get_id_for_uid(mm: &ModelManager, uid: Uuid) -> Result<Id> {
		base::get_id_for_uid::<Self>(mm, uid)
	}
}
