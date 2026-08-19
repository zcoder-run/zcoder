use std::path::Path;
use simple_fs::{SPath, ensure_file_dir};
use crate::Result;

pub const CACHE_DIR: &str = ".zcoder/.cache";

/// Save content to a file relative to the `.zcoder/.cache/` directory.
pub fn save_file_cache(file_path: impl AsRef<Path>, content: impl AsRef<[u8]>) -> Result<SPath> {
	let full_path = SPath::new(CACHE_DIR).join_std_path(file_path.as_ref())?;
	ensure_file_dir(&full_path)?;
	std::fs::write(full_path.as_std_path(), content.as_ref())?;
	Ok(full_path)
}

// region:    --- Tests

#[cfg(test)]
mod tests {
	type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>;

	use super::*;

	#[test]
	fn test_cache_save_file_cache_simple() -> Result<()> {
		// -- Setup & Fixtures
		let rel_path = "test-output/test_cache.txt";
		let content = b"hello zcoder cache";

		// -- Exec
		let saved_path = save_file_cache(rel_path, content)?;

		// -- Check
		assert!(saved_path.exists());
		let read_content = std::fs::read(saved_path.as_std_path())?;
		assert_eq!(read_content, content);

		// Cleanup
		// let _ = std::fs::remove_file(saved_path.as_std_path());

		Ok(())
	}
}

// endregion: --- Tests
