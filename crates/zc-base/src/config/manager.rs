use crate::config::{Config, ConfigInner, DEFAULT_CONFIG_TOML, Error, Result};
use crate::model::{Id, ModelManager, WksBmc};
use arc_swap::ArcSwap;
use simple_fs::SPath;
use std::collections::HashMap;
use std::fs;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

// region:    --- Types

pub struct ConfigManager {
	config_path: SPath,
	current: ArcSwap<ConfigInner>,
	last_mtime: Mutex<Option<SystemTime>>,
	wks_configs: Mutex<HashMap<Id, CachedWksConfig>>,
}

struct CachedWksConfig {
	#[allow(dead_code)]
	wks_config_path: SPath,
	last_wks_mtime: Option<SystemTime>,
	last_base_mtime: Option<SystemTime>,
	config: Config,
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
			if let Some(parent) = config_path.parent() {
				let _ = simple_fs::ensure_dir(parent);
			}
			let _ = fs::write(&config_path, DEFAULT_CONFIG_TOML);
			let inner = ConfigInner::from_toml_str(DEFAULT_CONFIG_TOML)?;
			let mtime = fs::metadata(&config_path).ok().and_then(|m| m.modified().ok());
			(inner, mtime)
		};

		Ok(Self {
			config_path,
			current: ArcSwap::from_pointee(inner),
			last_mtime: Mutex::new(mtime),
			wks_configs: Mutex::new(HashMap::new()),
		})
	}

	pub fn get_config(&self) -> Config {
		let inner = self.current.load_full();
		Config::from(inner)
	}

	pub async fn resolve_for_wks(&self, mm: &ModelManager, wks_id: Option<Id>) -> Result<Config> {
		let Some(wks_id) = wks_id else {
			self.refresh_if_modified()?;
			return Ok(self.get_config());
		};

		if wks_id == Id::default() {
			self.refresh_if_modified()?;
			return Ok(self.get_config());
		}

		let wks = match WksBmc::get(mm, wks_id).await {
			Ok(wks) => wks,
			Err(_) => {
				tracing::warn!("->> unknown wks_id '{wks_id}', falling back to base config");
				self.refresh_if_modified()?;
				return Ok(self.get_config());
			}
		};

		let wks_dir = SPath::from(wks.dir);
		self.resolve_for_wks_dir(wks_id, &wks_dir)
	}

	pub fn resolve_for_wks_dir(&self, wks_id: Id, wks_dir: &SPath) -> Result<Config> {
		self.refresh_if_modified()?;

		let base_mtime = *self.last_mtime.lock().map_err(|_| Error::custom("lock poisoned"))?;
		let wks_config_path = wks_dir.join(".zcoder").join("config.toml");

		let wks_mtime = if wks_config_path.exists() {
			fs::metadata(&wks_config_path).ok().and_then(|m| m.modified().ok())
		} else {
			None
		};

		// Check cache
		{
			let wks_cache = self.wks_configs.lock().map_err(|_| Error::custom("lock poisoned"))?;
			if let Some(cached) = wks_cache.get(&wks_id)
				&& cached.last_base_mtime == base_mtime
				&& cached.last_wks_mtime == wks_mtime
			{
				return Ok(cached.config.clone());
			}
		}

		// Compute layered config
		let layered_config = if wks_config_path.exists() {
			let base_toml = if self.config_path.exists() {
				fs::read_to_string(&self.config_path)?
			} else {
				DEFAULT_CONFIG_TOML.to_string()
			};
			let wks_toml = fs::read_to_string(&wks_config_path)?;
			Config::layer_toml_strs(&base_toml, &wks_toml)?
		} else {
			self.get_config()
		};

		// Update cache
		{
			let mut wks_cache = self.wks_configs.lock().map_err(|_| Error::custom("lock poisoned"))?;
			wks_cache.insert(
				wks_id,
				CachedWksConfig {
					wks_config_path,
					last_wks_mtime: wks_mtime,
					last_base_mtime: base_mtime,
					config: layered_config.clone(),
				},
			);
		}

		Ok(layered_config)
	}

	pub fn refresh_if_modified(&self) -> Result<bool> {
		if !self.config_path.exists() {
			if let Some(parent) = self.config_path.parent() {
				let _ = simple_fs::ensure_dir(parent);
			}
			let _ = fs::write(&self.config_path, DEFAULT_CONFIG_TOML);
			let new_inner = ConfigInner::from_toml_str(DEFAULT_CONFIG_TOML)?;
			let current_mtime = fs::metadata(&self.config_path).ok().and_then(|m| m.modified().ok());

			let mut last_mtime_guard = self
				.last_mtime
				.lock()
				.map_err(|_| crate::config::Error::custom("ConfigManager lock poisoned"))?;

			self.current.store(Arc::new(new_inner));
			*last_mtime_guard = current_mtime;

			return Ok(true);
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
		assert!(tmp_path.exists());
		assert_eq!(config.maestro_model(), "$small");
		assert_eq!(config.get_model("$small")?, "gemini-3.5-flash-lite");
		assert!(!manager.refresh_if_modified()?);

		// Clean up test file
		let _ = fs::remove_file(&tmp_path);

		// Refreshing when deleted should recreate file with defaults
		assert!(manager.refresh_if_modified()?);
		assert!(tmp_path.exists());
		assert_eq!(manager.get_config().get_model("$small")?, "gemini-3.5-flash-lite");

		let _ = fs::remove_file(&tmp_path);
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

	#[tokio::test]
	async fn test_config_manager_resolve_for_wks() -> Result<()> {
		let mm = crate::model::get_model_manager()?;

		let tmp_base_path =
			SPath::from_std_path_buf(std::env::temp_dir())?.join(format!("zc_test_base_{}.toml", uuid::Uuid::new_v4()));
		let base_toml = r#"
[maestro]
model = "$small"

[model_sizes]
small = "lite"

[model_aliases]
lite = "base-lite"
my_alias = "base-target"
"#;
		fs::write(&tmp_base_path, base_toml)?;
		let manager = ConfigManager::from_file(&tmp_base_path)?;

		// 1. Base-only config resolves
		let base_resolved = manager.resolve_for_wks(mm, None).await?;
		assert_eq!(base_resolved.get_model("my_alias")?, "base-target");

		// 2. Workspace config overrides model alias
		let tmp_wks_dir =
			SPath::from_std_path_buf(std::env::temp_dir())?.join(format!("zc_test_wks_{}", uuid::Uuid::new_v4()));
		let wks_dot_dir = tmp_wks_dir.join(".zcoder");
		simple_fs::ensure_dir(&wks_dot_dir)?;
		let wks_toml = r#"
[model_aliases]
my_alias = "wks-override-target"
"#;
		fs::write(wks_dot_dir.join("config.toml"), wks_toml)?;
		let wks_id = WksBmc::get_or_create_by_dir(mm, tmp_wks_dir.as_str(), None).await?;

		let wks_resolved = manager.resolve_for_wks(mm, Some(wks_id)).await?;
		assert_eq!(wks_resolved.get_model("my_alias")?, "wks-override-target");
		assert_eq!(wks_resolved.get_model("lite")?, "base-lite");

		// Verify cache hit
		let wks_resolved_cached = manager.resolve_for_wks(mm, Some(wks_id)).await?;
		assert_eq!(wks_resolved_cached.get_model("my_alias")?, "wks-override-target");

		// 3. Unknown wks_id falls back to base config with a warning
		let unknown_id = Id::try_from("11111111-2222-3333-4444-555555555555".to_string())?;
		let fallback_resolved = manager.resolve_for_wks(mm, Some(unknown_id)).await?;
		assert_eq!(fallback_resolved.get_model("my_alias")?, "base-target");

		let _ = fs::remove_file(&tmp_base_path);
		let _ = fs::remove_dir_all(&tmp_wks_dir);
		Ok(())
	}
}

// endregion: --- Tests
