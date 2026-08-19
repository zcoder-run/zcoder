use crate::model::EntityType;

pub trait DbBmc: Sized {
	const TABLE: &'static str;
	const ENTITY_TYPE: EntityType;

	fn table_ref() -> &'static str {
		Self::TABLE
	}
}
