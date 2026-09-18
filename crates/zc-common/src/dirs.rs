//! Path helpers derived from the shared constants in [`crate::consts`].

// region:    --- Modules

use crate::consts::{CONFIG_DIR_NAME, DEBUG_LOG_DIR_NAME, DEBUG_LOG_FILE_NAME, WKS_MARKER_DIR_NAME, ZBASE_DIR_NAME};
use crate::{Error, Result};
use simple_fs::SPath;

// endregion: --- Modules

// region:    --- Paths

/// Returns the `zc base` home directory, `$HOME/.config/zcoder-base`.
pub fn zbase_dir() -> Result<SPath> {
	let home = home_dir().ok_or_else(|| Error::custom("HOME environment variable is not set"))?;
	Ok(zbase_dir_from_home(&home))
}

/// Returns the `zc base` log file, `$HOME/.config/zcoder-base/debug-log/log.txt`.
pub fn zbase_log_file() -> Result<SPath> {
	Ok(zbase_dir()?.join(DEBUG_LOG_DIR_NAME).join(DEBUG_LOG_FILE_NAME))
}

/// Walks up from `from` and returns the first directory that contains a `.zcoder/` marker.
pub fn find_wks_dir(from: &SPath) -> Option<SPath> {
	let mut current = Some(from.clone());
	while let Some(dir) = current {
		if dir.join(WKS_MARKER_DIR_NAME).is_dir() {
			return Some(dir);
		}
		current = dir.parent();
	}
	None
}

/// Returns the workspace log file, `<wks_dir>/.zcoder/debug-log/log.txt`.
pub fn wks_log_file(wks_dir: &SPath) -> SPath {
	wks_dir
		.join(WKS_MARKER_DIR_NAME)
		.join(DEBUG_LOG_DIR_NAME)
		.join(DEBUG_LOG_FILE_NAME)
}

// endregion: --- Paths

// region:    --- Support

fn home_dir() -> Option<SPath> {
	std::env::var("HOME").ok().map(SPath::from)
}

fn zbase_dir_from_home(home: &SPath) -> SPath {
	home.join(CONFIG_DIR_NAME).join(ZBASE_DIR_NAME)
}

// endregion: --- Support

// region:    --- Tests

#[cfg(test)]
mod tests {
	type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>;

	use super::*;
	use std::fs;

	fn new_test_root(name: &str) -> Result<SPath> {
		let temp_dir = SPath::from_std_path_buf(std::env::temp_dir())?;
		Ok(temp_dir.join(format!("zc-common-dirs-{name}-{}", std::process::id())))
	}

	#[test]
	fn test_dirs_find_wks_dir_found() -> Result<()> {
		// -- Setup & Fixtures
		let root = new_test_root("found")?;
		let wks_dir = root.join("proj");
		fs::create_dir_all(wks_dir.join(WKS_MARKER_DIR_NAME))?;

		// -- Exec
		let found = find_wks_dir(&wks_dir);

		// -- Check
		assert_eq!(found, Some(wks_dir.clone()));

		// -- Cleanup
		fs::remove_dir_all(&root)?;

		Ok(())
	}

	#[test]
	fn test_dirs_find_wks_dir_not_found() -> Result<()> {
		// -- Setup & Fixtures
		let root = new_test_root("not-found")?;
		let nested = root.join("plain").join("nested");
		fs::create_dir_all(&nested)?;

		// -- Exec
		let found = find_wks_dir(&nested);

		// -- Check
		assert_eq!(found, None);

		// -- Cleanup
		fs::remove_dir_all(&root)?;

		Ok(())
	}

	#[test]
	fn test_dirs_find_wks_dir_nested_start() -> Result<()> {
		// -- Setup & Fixtures
		let root = new_test_root("nested")?;
		let wks_dir = root.join("proj");
		fs::create_dir_all(wks_dir.join(WKS_MARKER_DIR_NAME))?;
		let nested_start = wks_dir.join("src").join("deep");
		fs::create_dir_all(&nested_start)?;

		// -- Exec
		let found = find_wks_dir(&nested_start);

		// -- Check
		assert_eq!(found, Some(wks_dir.clone()));

		// -- Cleanup
		fs::remove_dir_all(&root)?;

		Ok(())
	}

	#[test]
	fn test_dirs_wks_log_file() -> Result<()> {
		// -- Setup & Fixtures
		let wks_dir = SPath::from("/home/dev/proj");

		// -- Exec
		let log_file = wks_log_file(&wks_dir);

		// -- Check
		assert_eq!(log_file.as_str(), "/home/dev/proj/.zcoder/debug-log/log.txt");

		Ok(())
	}

	#[test]
	fn test_dirs_zbase_dir_from_home() -> Result<()> {
		// -- Setup & Fixtures
		let home = SPath::from("/home/dev");

		// -- Exec
		let zbase_dir = zbase_dir_from_home(&home);

		// -- Check
		assert_eq!(zbase_dir.as_str(), "/home/dev/.config/zcoder-base");

		Ok(())
	}
}

// endregion: --- Tests
