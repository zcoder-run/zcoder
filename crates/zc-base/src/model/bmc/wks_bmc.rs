use crate::model::support::{self, DbBmc};
use crate::model::{EntityType, Id, ListWksOptions, ModelManager, Result, Wks, WksForCreate};
use modql::field::{Fields, HasSqliteFields};
use std::path::Path;

// region:    --- Bmc

pub struct WksBmc;

impl DbBmc for WksBmc {
	const TABLE: &'static str = "wks";
	const ENTITY_TYPE: EntityType = EntityType::Wks;
}

/// Basic CRUD
impl WksBmc {
	#[allow(unused)]
	pub async fn create(mm: &ModelManager, wks_c: WksForCreate) -> Result<Id> {
		let fields = wks_c.sqlite_not_none_fields();
		support::create::<Self>(mm, fields).await
	}

	pub async fn get(mm: &ModelManager, id: Id) -> Result<Wks> {
		support::get::<Self, _>(mm, id).await
	}

	#[allow(unused)]
	pub async fn list(mm: &ModelManager, list_options: Option<ListWksOptions>) -> Result<Vec<Wks>> {
		support::list::<Self, _>(mm, list_options, None).await
	}

	/// Returns the workspace row for the given directory, if any.
	pub async fn get_by_dir(mm: &ModelManager, dir: impl Into<String>) -> Result<Option<Wks>> {
		let dir = normalize_dir(dir.into())?;
		let filter_fields = WksDirFilter { dir: Some(dir) }.sqlite_not_none_fields();
		support::first::<Self, Wks>(mm, None, Some(filter_fields)).await
	}

	/// Resolves a workspace directory to its `wks_id`, creating the row when missing.
	///
	/// The directory is normalized to an absolute path first, so two clients
	/// spelling the same workspace differently resolve to one row.
	pub async fn get_or_create_by_dir(
		mm: &ModelManager,
		dir: impl Into<String>,
		label: Option<String>,
	) -> Result<Id> {
		let dir = normalize_dir(dir.into())?;

		// -- Fast path: the directory is already known
		if let Some(wks) = Self::get_by_dir(mm, &dir).await? {
			return Ok(wks.id);
		}

		// -- Insert only when still missing, so two clients racing on the same dir cannot both insert
		let not_exists_fields = WksDirFilter { dir: Some(dir.clone()) }.sqlite_not_none_fields();
		let fields = WksForCreate {
			dir: dir.clone(),
			label,
		}
		.sqlite_not_none_fields();
		if let Some(id) = support::create_where_not_exists::<Self>(mm, fields, not_exists_fields, None).await? {
			return Ok(id);
		}

		// -- The row appeared between the check and the insert, so read it back
		let wks = Self::get_by_dir(mm, &dir)
			.await?
			.ok_or_else(|| format!("Cannot resolve wks for dir: {dir}"))?;
		Ok(wks.id)
	}
}

// endregion: --- Bmc

// region:    --- Support

/// Normalizes a workspace directory to an absolute path, so two clients spelling
/// the same workspace differently resolve to one row.
fn normalize_dir(dir: String) -> Result<String> {
	let path = Path::new(&dir);
	let abs = if path.is_absolute() {
		path.to_path_buf()
	} else {
		std::env::current_dir()
			.map_err(|err| format!("Cannot resolve current dir: {err}"))?
			.join(path)
	};
	let normalized = std::fs::canonicalize(&abs).unwrap_or(abs);
	Ok(normalized.to_string_lossy().to_string())
}

/// Directory-only projection used to build the lookup filter and the
/// not-exists clause, so the label never participates in the identity lookup.
#[derive(Debug, Clone, Fields)]
struct WksDirFilter {
	dir: Option<String>,
}

// endregion: --- Support

// region:    --- Tests

#[cfg(test)]
mod tests {
	type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>;

	use super::*;
	use crate::model::get_model_manager;

	#[tokio::test]
	async fn test_model_wks_bmc_get_or_create_same_dir_same_id() -> Result<()> {
		// -- Setup & Fixtures
		let mm = get_model_manager()?;

		// -- Exec
		let id_first = WksBmc::get_or_create_by_dir(mm, "/tmp/zc-test-wks-same-dir", None).await?;
		let id_second = WksBmc::get_or_create_by_dir(mm, "/tmp/zc-test-wks-same-dir", None).await?;

		// -- Check
		assert_eq!(id_first, id_second);

		Ok(())
	}

	#[tokio::test]
	async fn test_model_wks_bmc_get_or_create_diff_dir_diff_id() -> Result<()> {
		// -- Setup & Fixtures
		let mm = get_model_manager()?;

		// -- Exec
		let id_a = WksBmc::get_or_create_by_dir(mm, "/tmp/zc-test-wks-dir-a", None).await?;
		let id_b = WksBmc::get_or_create_by_dir(mm, "/tmp/zc-test-wks-dir-b", None).await?;

		// -- Check
		assert_ne!(id_a, id_b);

		Ok(())
	}

	#[tokio::test]
	async fn test_model_wks_bmc_get_or_create_keeps_label() -> Result<()> {
		// -- Setup & Fixtures
		let mm = get_model_manager()?;

		// -- Exec
		let id = WksBmc::get_or_create_by_dir(mm, "/tmp/zc-test-wks-label", Some("dev/zcoder".to_string())).await?;

		// -- Check
		let wks = WksBmc::get(mm, id).await?;
		assert_eq!(wks.id, id);
		assert_eq!(wks.label.as_deref(), Some("dev/zcoder"));

		Ok(())
	}
}

// endregion: --- Tests
