use crate::config::{Config, ConfigInner, Result};
use arc_swap::ArcSwap;
use simple_fs::SPath;
use std::fs;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

// region:    --- Types

pub struct ConfigManager {
	config_path: SPath,
	current: ArcSwap<ConfigInner>,
	last_mtime: Mutex<Option<SystemTime>>,
}

// endregion: --- Types

// region:    --- ConfigManager

impl ConfigManager {
	pub fn from_file(config_path: impl Into<SPath>) -> Result<Self> {
		let config_path = config_path.into();

		let (inner, mtime) = if config_path.exists() {
			let metadata = fs::metadata(&config_path)?;
			let mtime = metadata.modified().ok();
			let content = fs::read_to_string(&config_path)?;
			let inner = ConfigInner::from_toml_str(&content)?;
			(inner, mtime)
		} else {
			(ConfigInner::default(), None)
		};

		Ok(Self {
			config_path,
			current: ArcSwap::from_pointee(inner),
			last_mtime: Mutex::new(mtime),
		})
	}

	pub fn get_config(&self) -> Config {
		let inner = self.current.load_full();
		Config::from(inner)
	}

	pub fn refresh_if_modified(&self) -> Result<bool> {
		if !self.config_path.exists() {
			return Ok(false);
		}

		let metadata = fs::metadata(&self.config_path)?;
		let current_mtime = metadata.modified().ok();

		let mut last_mtime_guard = self
			.last_mtime
			.lock()
			.map_err(|_| crate::config::Error::custom("ConfigManager lock poisoned"))?;

		if current_mtime.is_some() && current_mtime == *last_mtime_guard {
			return Ok(false);
		}

		let content = fs::read_to_string(&self.config_path)?;
		let new_inner = ConfigInner::from_toml_str(&content)?;

		self.current.store(Arc::new(new_inner));
		*last_mtime_guard = current_mtime;

		Ok(true)
	}

	pub fn config_path(&self) -> &SPath {
		&self.config_path
	}
}

// endregion: --- ConfigManager

// region:    --- Tests

#[cfg(test)]
mod tests {
	type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>;

	use super::*;
	use simple_fs::SPath;
	use std::fs;
	use std::thread::sleep;
	use std::time::Duration;

	#[test]
	fn test_config_manager_non_existent_file() -> Result<()> {
		// -- Setup & Fixtures
		let tmp_path = SPath::from_std_path_buf(std::env::temp_dir())?
			.join(format!("zc_test_non_existent_{}.toml", uuid::Uuid::new_v4()));

		// -- Exec
		let manager = ConfigManager::from_file(&tmp_path)?;
		let config = manager.get_config();

		// -- Check
		assert_eq!(config.maestro_model(), "$small");
		assert!(!manager.refresh_if_modified()?);

		Ok(())
	}

	#[test]
	fn test_config_manager_load_and_reload() -> Result<()> {
		// -- Setup & Fixtures
		let tmp_path = SPath::from_std_path_buf(std::env::temp_dir())?
			.join(format!("zc_test_reload_{}.toml", uuid::Uuid::new_v4()));

		let initial_toml = r#"
[maestro]
model = "$small"

[model_sizes]
small = "lite"

[model_aliases]
lite = "gemini-3.5-flash-lite"
"#;
		fs::write(&tmp_path, initial_toml)?;

		// -- Exec
		let manager = ConfigManager::from_file(&tmp_path)?;
		let initial_config = manager.get_config();

		// -- Check
		assert_eq!(initial_config.get_model("$small")?, "gemini-3.5-flash-lite");

		// -- Update file on disk
		sleep(Duration::from_millis(50));
		let updated_toml = r#"
[maestro]
model = "$big"

[model_sizes]
small = "lite"
big = "sol"

[model_aliases]
lite = "gemini-3.5-flash-lite"
sol = "gpt-5.6-sol"
"#;
		fs::write(&tmp_path, updated_toml)?;

		let reloaded = manager.refresh_if_modified()?;
		assert!(reloaded);

		let updated_config = manager.get_config();
		assert_eq!(updated_config.get_model("$big")?, "gpt-5.6-sol");

		// -- Check syntax error preserves previous config
		sleep(Duration::from_millis(50));
		fs::write(&tmp_path, "invalid toml [[")?;
		let reload_err = manager.refresh_if_modified();
		assert!(reload_err.is_err());

		let retained_config = manager.get_config();
		assert_eq!(retained_config.get_model("$big")?, "gpt-5.6-sol");

		// Clean up test file
		let _ = fs::remove_file(&tmp_path);

		Ok(())
	}
}

// endregion: --- Tests
