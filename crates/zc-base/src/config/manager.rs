use crate::config::{Config, ConfigInner, Error, Result};
use crate::model::{Id, ModelManager, WksBmc};
use arc_swap::ArcSwap;
use simple_fs::SPath;
use std::fs;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::SystemTime;

// region:    --- Types

pub struct ConfigManager {
	config_path: SPath,
	default_config_path: Option<SPath>,
	current: ArcSwap<ConfigInner>,
	last_mtimes: Mutex<BaseMtimes>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct BaseMtimes {
	default_config_mtime: Option<SystemTime>,
	config_mtime: Option<SystemTime>,
}

// endregion: --- Types

// region:    --- ConfigManager

impl ConfigManager {
	pub fn from_file(config_path: impl Into<SPath>) -> Result<Self> {
		let config_path = config_path.into();

		if !config_path.exists() {
			if let Some(parent) = config_path.parent() {
				let _ = simple_fs::ensure_dir(parent);
			}
			let _ = fs::write(&config_path, embedded_default_config_toml()?);
		}

		Self::build(config_path, None)
	}

	/// Creates a manager backed by the base directory configuration layers.
	///
	/// The layers are `config-default.toml` followed by `config-user.toml`.
	pub fn from_zbase_dir(zbase_dir: impl Into<SPath>) -> Result<Self> {
		let zbase_dir = zbase_dir.into();
		let config_path = zbase_dir.join(BASE_USER_FILE_NAME);
		let default_config_path = zbase_dir.join(BASE_DEFAULT_FILE_NAME);

		Self::build(config_path, Some(default_config_path))
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

	/// Resolves the effective configuration for a workspace directory, layering
	/// the base configs and the workspace config fresh from disk on every call.
	pub fn resolve_for_wks_dir(&self, wks_id: Id, wks_dir: &SPath) -> Result<Config> {
		tracing::debug!("->> resolving fresh config for wks_id {wks_id}");
		let wks_config_path = wks_dir.join(".zcoder").join("config.toml");
		self.layer_wks_config(&wks_config_path)
	}

	pub fn refresh_if_modified(&self) -> Result<bool> {
		if !self.config_path.exists() {
			if let Some(parent) = self.config_path.parent() {
				let _ = simple_fs::ensure_dir(parent);
			}
			let _ = fs::write(&self.config_path, embedded_default_config_toml()?);
			let layers = base_layer_strs(&self.config_path, self.default_config_path.as_ref())?;
			let new_inner = layer_strs_to_inner(&layers)?;
			let current_mtimes = read_base_mtimes(&self.config_path, self.default_config_path.as_ref());

			let mut last_mtimes_guard = self
				.last_mtimes
				.lock()
				.map_err(|_| crate::config::Error::custom("ConfigManager lock poisoned"))?;

			self.current.store(Arc::new(new_inner));
			*last_mtimes_guard = current_mtimes;

			return Ok(true);
		}

		let current_mtimes = read_base_mtimes(&self.config_path, self.default_config_path.as_ref());

		let mut last_mtimes_guard = self
			.last_mtimes
			.lock()
			.map_err(|_| crate::config::Error::custom("ConfigManager lock poisoned"))?;

		if current_mtimes.config_mtime.is_some() && current_mtimes == *last_mtimes_guard {
			return Ok(false);
		}

		let layers = base_layer_strs(&self.config_path, self.default_config_path.as_ref())?;
		let new_inner = layer_strs_to_inner(&layers)?;

		self.current.store(Arc::new(new_inner));
		*last_mtimes_guard = current_mtimes;

		Ok(true)
	}

	pub fn config_path(&self) -> &SPath {
		&self.config_path
	}
}

// endregion: --- ConfigManager

// region:    --- Support

impl ConfigManager {
	fn build(config_path: SPath, default_config_path: Option<SPath>) -> Result<Self> {
		let layers = base_layer_strs(&config_path, default_config_path.as_ref())?;
		let inner = layer_strs_to_inner(&layers)?;
		let mtimes = read_base_mtimes(&config_path, default_config_path.as_ref());

		Ok(Self {
			config_path,
			default_config_path,
			current: ArcSwap::from_pointee(inner),
			last_mtimes: Mutex::new(mtimes),
		})
	}

	fn layer_wks_config(&self, wks_config_path: &SPath) -> Result<Config> {
		let mut layers = base_layer_strs(&self.config_path, self.default_config_path.as_ref())?;
		if wks_config_path.exists() {
			layers.push(fs::read_to_string(wks_config_path)?);
		}
		layer_strs_to_config(&layers)
	}
}

fn base_layer_strs(config_path: &SPath, default_config_path: Option<&SPath>) -> Result<Vec<String>> {
	let mut layers: Vec<String> = Vec::new();

	if let Some(default_config_path) = default_config_path
		&& default_config_path.exists()
	{
		layers.push(fs::read_to_string(default_config_path)?);
	}
	if config_path.exists() {
		layers.push(fs::read_to_string(config_path)?);
	}
	if layers.is_empty() {
		layers.push(embedded_default_config_toml()?.to_string());
	}

	Ok(layers)
}

fn layer_strs_to_config(layers: &[String]) -> Result<Config> {
	let inner = layer_strs_to_inner(layers)?;
	Ok(Config::from(inner))
}

fn layer_strs_to_inner(layers: &[String]) -> Result<ConfigInner> {
	let layer_refs: Vec<&str> = layers.iter().map(|layer| layer.as_str()).collect();
	ConfigInner::layer_toml_strs_layers(&layer_refs)
}

fn read_base_mtimes(config_path: &SPath, default_config_path: Option<&SPath>) -> BaseMtimes {
	BaseMtimes {
		default_config_mtime: default_config_path
			.and_then(|path| fs::metadata(path).ok())
			.and_then(|metadata| metadata.modified().ok()),
		config_mtime: fs::metadata(config_path).ok().and_then(|metadata| metadata.modified().ok()),
	}
}

/// Returns the embedded default base configuration content, cached after the first read.
fn embedded_default_config_toml() -> Result<&'static str> {
	static DEFAULT_CONFIG_TOML: OnceLock<String> = OnceLock::new();

	if let Some(content) = DEFAULT_CONFIG_TOML.get() {
		return Ok(content.as_str());
	}

	let content = zc_asset::extract_asset_str(DEFAULT_CONFIG_ASSET).map_err(Error::custom_from_err)?;
	let content = DEFAULT_CONFIG_TOML.get_or_init(|| content);
	Ok(content.as_str())
}

const BASE_DEFAULT_FILE_NAME: &str = "config-default.toml";
const BASE_USER_FILE_NAME: &str = "config-user.toml";
const DEFAULT_CONFIG_ASSET: &str = "base/config-default.toml";

// endregion: --- Support

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

	#[test]
	fn test_config_manager_from_zbase_dir_layers() -> Result<()> {
		// -- Setup & Fixtures
		let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_nanos();
		let tmp_root = SPath::from_std_path_buf(std::env::temp_dir())?.join(format!("zc_test_zbase_layers_{nanos}"));
		let zbase_dir = tmp_root.join("zbase");
		fs::create_dir_all(&zbase_dir)?;

		let default_toml = r#"
[maestro]
model = "$small"

[model_sizes]
small = "lite"

[model_aliases]
lite = "gemini-3.5-flash-lite"
flash = "default-flash"
"#;
		fs::write(zbase_dir.join("config-default.toml"), default_toml)?;

		let user_toml = r#"
[model_aliases]
flash = "user-flash"
"#;
		fs::write(zbase_dir.join("config-user.toml"), user_toml)?;

		// -- Exec
		let manager = ConfigManager::from_zbase_dir(&zbase_dir)?;

		// -- Check base layers
		let base_config = manager.get_config();
		assert_eq!(base_config.get_model("lite")?, "gemini-3.5-flash-lite");
		assert_eq!(base_config.get_model("flash")?, "user-flash");
		assert_eq!(base_config.get_model("$small")?, "gemini-3.5-flash-lite");

		// -- Exec & Check workspace layer on top of base layers
		let wks_dir = tmp_root.join("wks");
		let zcoder_dir = wks_dir.join(".zcoder");
		fs::create_dir_all(&zcoder_dir)?;
		let wks_toml = r#"
[model_aliases]
flash = "wks-flash"
"#;
		fs::write(zcoder_dir.join("config.toml"), wks_toml)?;

		let wks_config = manager.resolve_for_wks_dir(Id::default(), &wks_dir)?;

		// -- Check
		assert_eq!(wks_config.get_model("flash")?, "wks-flash");
		assert_eq!(wks_config.get_model("lite")?, "gemini-3.5-flash-lite");
		assert_eq!(wks_config.get_model("$small-high")?, "gemini-3.5-flash-lite-high");

		// -- Cleanup
		let _ = fs::remove_dir_all(&tmp_root);

		Ok(())
	}

	#[test]
	fn test_config_manager_resolve_for_wks_dir_fresh_picks_up_edits() -> Result<()> {
		// -- Setup & Fixtures
		let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_nanos();
		let tmp_root = SPath::from_std_path_buf(std::env::temp_dir())?.join(format!("zc_test_fresh_pickup_{nanos}"));
		let zbase_dir = tmp_root.join("zbase");
		fs::create_dir_all(&zbase_dir)?;

		let default_toml = r#"
[model_aliases]
flash = "default-flash"
"#;
		fs::write(zbase_dir.join("config-default.toml"), default_toml)?;
		fs::write(zbase_dir.join("config-user.toml"), "")?;

		let manager = ConfigManager::from_zbase_dir(&zbase_dir)?;

		let wks_dir = tmp_root.join("wks");
		let zcoder_dir = wks_dir.join(".zcoder");
		fs::create_dir_all(&zcoder_dir)?;
		let wks_toml = r#"
[model_aliases]
flash = "wks-flash"
"#;
		fs::write(zcoder_dir.join("config.toml"), wks_toml)?;

		// -- Exec & Check the workspace layer wins
		let config_first = manager.resolve_for_wks_dir(Id::default(), &wks_dir)?;
		assert_eq!(config_first.get_model("flash")?, "wks-flash");

		// -- Edit the user layer on disk without any refresh call and resolve again
		fs::write(zbase_dir.join("config-user.toml"), "[model_aliases]\nflash = \"user-flash\"\n")?;
		let config_second = manager.resolve_for_wks_dir(Id::default(), &wks_dir)?;
		assert_eq!(config_second.get_model("flash")?, "wks-flash");

		// -- Remove the workspace layer so the fresh user layer wins
		fs::remove_file(zcoder_dir.join("config.toml"))?;
		let config_third = manager.resolve_for_wks_dir(Id::default(), &wks_dir)?;
		assert_eq!(config_third.get_model("flash")?, "user-flash");

		// -- Cleanup
		let _ = fs::remove_dir_all(&tmp_root);

		Ok(())
	}

	#[test]
	fn test_config_manager_from_file_uses_embedded_asset() -> Result<()> {
		// -- Setup & Fixtures
		let tmp_path = SPath::from_std_path_buf(std::env::temp_dir())?
			.join(format!("zc_test_embedded_default_{}.toml", uuid::Uuid::new_v4()));

		// -- Exec
		ConfigManager::from_file(&tmp_path)?;

		// -- Check
		let written = fs::read_to_string(&tmp_path)?;
		let embedded = zc_asset::extract_asset_str("base/config-default.toml")?;
		assert_eq!(written, embedded);

		// -- Clean
		let _ = fs::remove_file(&tmp_path);

		Ok(())
	}
}

// endregion: --- Tests
