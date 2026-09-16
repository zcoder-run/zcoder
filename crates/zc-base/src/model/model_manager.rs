use crate::model::Result;
use crate::model::db::Db;
use std::sync::OnceLock;

#[derive(Debug)]
pub struct ModelManager {
	db: Db,
}

/// Internal Constructors
static INSTANCE: OnceLock<Result<ModelManager>> = OnceLock::new();

pub fn get_model_manager() -> std::result::Result<&'static ModelManager, String> {
	let res = INSTANCE.get_or_init(ModelManager::new);
	res.as_ref().map_err(|err| err.to_string())
}

/// Management
impl ModelManager {
	/// NOTE: This is to make sure the db does not become too big in memory
	///      For now, very agressive, just delete everything.
	/// Should be called at the start of each run
	pub async fn trim(&self) -> Result<usize> {
		let db = self.db();
		let run_count = db.exec("DELETE FROM run", []).await?;

		Ok(run_count)
	}

	pub async fn db_size(&self) -> Result<i64> {
		let db = self.db();
		let sql = r#"
SELECT page_count * page_size as size_bytes
FROM pragma_page_count(), pragma_page_size();
		"#;

		let res = db.exec_returning_num(sql, ()).await?;
		Ok(res)
	}
}

impl ModelManager {
	fn new() -> Result<Self> {
		let db = Db::new()?;
		Ok(Self { db })
	}
}

/// Getters
impl ModelManager {
	pub fn db(&self) -> &Db {
		&self.db
	}
}
