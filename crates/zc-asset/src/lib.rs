//! Embedded assets runtime for zcoder.

// region:    --- Modules

mod error;

pub use error::{Error, Result};

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
		.map_err(|_| Error::AssetNotFound {
			path: path.to_string(),
		})?;

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
}

// endregion: --- Tests
