//! Embedded assets runtime for zcoder.

// region:    --- Modules

mod error;

pub use error::{Error, Result};
use std::path::Path;
use std::io::{Cursor, Read};
use zip::ZipArchive;

// endregion: --- Modules

pub const ASSETS_ZIP: &[u8] = include_bytes!(env!("ASSETS_ZIP"));

// region:    --- Types

#[derive(Debug, Clone)]
pub struct ZFile {
	pub path: String,
	pub content: Vec<u8>,
}

impl ZFile {
	pub fn as_str(&self) -> Result<&str> {
		Ok(std::str::from_utf8(&self.content)?)
	}

	pub fn into_string(self) -> Result<String> {
		Ok(String::from_utf8(self.content)?)
	}
}

// endregion: --- Types

// region:    --- Public APIs

/// Extract the binary content of an asset at the given path.
pub fn extract_asset(path: &str) -> Result<Vec<u8>> {
	let normalized_path = path.trim_start_matches('/');
	let mut archive = ZipArchive::new(Cursor::new(ASSETS_ZIP))?;
	let mut file = archive
		.by_name(normalized_path)
		.map_err(|_| Error::AssetNotFound { path: path.to_string() })?;

	let mut content = Vec::with_capacity(file.size() as usize);
	file.read_to_end(&mut content)?;
	Ok(content)
}

/// Extract the UTF-8 string content of an asset at the given path.
pub fn extract_asset_str(path: &str) -> Result<String> {
	let bytes = extract_asset(path)?;
	Ok(String::from_utf8(bytes)?)
}

/// Extract a `ZFile` containing path and binary content.
pub fn extract_zfile(path: &str) -> Result<ZFile> {
	let content = extract_asset(path)?;
	Ok(ZFile {
		path: path.to_string(),
		content,
	})
}

/// List all asset paths matching the optional prefix filter.
pub fn list_asset_paths(prefix: &str) -> Result<Vec<String>> {
	let normalized_prefix = prefix.trim_start_matches('/');
	let archive = ZipArchive::new(Cursor::new(ASSETS_ZIP))?;
	let mut paths = Vec::new();

	for file_name in archive.file_names() {
		if !file_name.ends_with('/') && file_name.starts_with(normalized_prefix) {
			paths.push(file_name.to_string());
		}
	}

	paths.sort();
	Ok(paths)
}

/// Initialize or update `.zcoder` directory in target project with missing embedded assets.
pub fn update_zcoder_project(project_dir: impl AsRef<Path>) -> Result<()> {
	let project_dir = project_dir.as_ref();
	let zcoder_dir = project_dir.join(".zcoder");
	if !zcoder_dir.exists() {
		std::fs::create_dir_all(&zcoder_dir)?;
	}

	let asset_paths = list_asset_paths("zcoder/")?;
	for asset_path in asset_paths {
		if let Some(rel_path) = asset_path.strip_prefix("zcoder/") {
			let dest_path = zcoder_dir.join(rel_path);
			if !dest_path.exists() {
				if let Some(parent) = dest_path.parent()
					&& !parent.exists()
				{
					std::fs::create_dir_all(parent)?;
				}
				let content = extract_asset(&asset_path)?;
				std::fs::write(&dest_path, content)?;
			}
		}
	}

	Ok(())
}

// endregion: --- Public APIs

// region:    --- Tests

#[cfg(test)]
mod tests {
	type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>;

	use super::*;

	#[test]
	fn test_asset_list_all() -> Result<()> {
		// -- Exec
		let paths = list_asset_paths("")?;

		// -- Check
		assert!(paths.contains(&"maestro/entry.tmpl".to_string()));
		Ok(())
	}

	#[test]
	fn test_asset_list_with_prefix() -> Result<()> {
		// -- Exec
		let paths = list_asset_paths("maestro")?;

		// -- Check
		assert_eq!(paths, vec!["maestro/entry.tmpl".to_string()]);
		Ok(())
	}

	#[test]
	fn test_asset_extract_str() -> Result<()> {
		// -- Exec
		let content = extract_asset_str("maestro/entry.tmpl")?;

		// -- Check
		assert!(content.contains("# Maestro Agent"));
		Ok(())
	}

	#[test]
	fn test_asset_extract_zfile() -> Result<()> {
		// -- Exec
		let zfile = extract_zfile("maestro/entry.tmpl")?;

		// -- Check
		assert_eq!(zfile.path, "maestro/entry.tmpl");
		assert!(zfile.as_str()?.contains("You are Maestro"));
		Ok(())
	}

	#[test]
	fn test_asset_not_found() -> Result<()> {
		// -- Exec
		let res = extract_asset("non_existent_file.txt");

		// -- Check
		assert!(res.is_err());
		Ok(())
	}

	#[test]
	fn test_asset_update_zcoder_project() -> Result<()> {
		// -- Setup & Fixtures
		let nanos = std::time::SystemTime::now()
			.duration_since(std::time::UNIX_EPOCH)?
			.as_nanos();
		let temp_dir = std::env::temp_dir().join(format!("zc_asset_test_{nanos}"));
		std::fs::create_dir_all(&temp_dir)?;

		// -- Exec
		update_zcoder_project(&temp_dir)?;

		// -- Check
		let config_path = temp_dir.join(".zcoder").join("config.toml");
		assert!(config_path.exists());
		let content = std::fs::read_to_string(&config_path)?;
		assert!(content.contains("[maestro]"));
		assert!(content.contains("[model_sizes]"));

		// Test non-overwrite behavior
		let custom_content = "# custom modification\n[maestro]\nmodel = 'custom'";
		std::fs::write(&config_path, custom_content)?;
		update_zcoder_project(&temp_dir)?;
		let content_after = std::fs::read_to_string(&config_path)?;
		assert_eq!(content_after, custom_content);

		// -- Clean
		let _ = std::fs::remove_dir_all(&temp_dir);
		Ok(())
	}
}

// endregion: --- Tests
