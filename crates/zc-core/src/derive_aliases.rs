use macro_rules_attribute::derive_alias;

derive_alias! {
	// Basic compare (no float)
	#[derive(Cmp!)] = #[derive(PartialEq, Eq, PartialOrd, Ord)];

	// Basic hash (e.g., for hash keys)
	//#[derive(Hash!)] = #[derive(PartialEq, Eq, Hash)];

	// For enum as str
	// #[derive(EnumAsStr!)] = #[derive(strum::IntoStaticStr, strum::AsRefStr)];
}

derive_alias! {
	// Scalar Struct type for DB Primitive type wrapper
	#[derive(ScalarStruct!)] = #[derive(
		crate::Cmp!,
		Clone,
		Copy,
		Hash,
		derive_more::From,
		derive_more::Into,
		derive_more::Display,
		derive_more::Deref,
		modql::SqliteFromValue,
		modql::SqliteToValue,
		serde::Serialize,
		serde::Deserialize,
	)];

	#[derive(ScalarEnum!)] = #[derive(
		crate::Cmp!,
		Clone,
		Copy,
		Hash,
		derive_more::Display,
		crate::EnumAsStr!,
		modql::SqliteFromValue,
		modql::SqliteToValue,
	)];
}
