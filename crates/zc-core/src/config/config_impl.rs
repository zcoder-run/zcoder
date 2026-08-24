use crate::config::{Error, Result};
use serde::{Deserialize, Serialize};
use simple_fs::SPath;
use std::collections::{BTreeMap, HashSet};
use std::ops::Deref;
use std::sync::Arc;

const MAX_ALIAS_DEPTH: usize = 16;

pub const DEFAULT_CONFIG_TOML: &str = r#"[workspace]
working_dir = "./"   # When relative, relative to cwd of the project_dir

[maestro]

model     = "$small"

[model_sizes]
# Addressed with `$` (model = "$small")
small     = "lite"
medium    = "flash"
big       = "sol"

[model_aliases]
# -- google
lite31        = "gemini-3.1-flash-lite"
lite          = "gemini-3.5-flash-lite"
flash         = "gemini-3.7-flash"
# -- Openai
luna          = "gpt-5.6-luna"
terra         = "gpt-5.6-terra"
sol           = "gpt-5.6-sol"
# -- Anthropic
opus          = "claude-opus-5"
claude        = "claude-sonnet-5"
sonnet        = "claude-sonnet-5"
haiku         = "claude-haiku-4-5"
"#;

// region:    --- Types

#[derive(Debug, Clone, Default)]
pub struct Config(Arc<ConfigInner>);

impl Deref for Config {
	type Target = ConfigInner;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(from = "ConfigToml", into = "ConfigToml")]
pub struct ConfigInner {
	pub maestro_model: Option<String>,

	pub workspace_working_dir: Option<SPath>,

	pub model_sizes: Option<BTreeMap<String, String>>,

	pub model_aliases: Option<BTreeMap<String, String>>,
}

// endregion: --- Types

// region:    --- Config

impl Config {
	pub fn from_toml_str(toml_str: &str) -> Result<Self> {
		let inner = ConfigInner::from_toml_str(toml_str)?;
		Ok(Self(Arc::new(inner)))
	}

	pub fn with_maestro_model(mut self, model: impl Into<String>) -> Self {
		Arc::make_mut(&mut self.0).maestro_model = Some(model.into());
		self
	}

	pub fn with_workspace_working_dir(mut self, dir: impl Into<SPath>) -> Self {
		Arc::make_mut(&mut self.0).workspace_working_dir = Some(dir.into());
		self
	}

	pub fn with_model_aliases(mut self, aliases: BTreeMap<String, String>) -> Self {
		Arc::make_mut(&mut self.0).model_aliases = Some(aliases);
		self
	}

	pub fn with_model_sizes(mut self, sizes: BTreeMap<String, String>) -> Self {
		Arc::make_mut(&mut self.0).model_sizes = Some(sizes);
		self
	}

	pub fn append_model_alias(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
		Arc::make_mut(&mut self.0)
			.model_aliases
			.get_or_insert_with(BTreeMap::new)
			.insert(key.into(), value.into());
		self
	}

	pub fn append_model_size(mut self, size: impl Into<String>, target: impl Into<String>) -> Self {
		Arc::make_mut(&mut self.0)
			.model_sizes
			.get_or_insert_with(BTreeMap::new)
			.insert(size.into(), target.into());
		self
	}

	pub fn maestro_model(&self) -> &str {
		self.0.maestro_model()
	}

	pub fn workspace_working_dir(&self) -> Option<&SPath> {
		self.0.workspace_working_dir()
	}

	pub fn model_sizes(&self) -> Option<&BTreeMap<String, String>> {
		self.0.model_sizes.as_ref()
	}

	pub fn model_aliases(&self) -> Option<&BTreeMap<String, String>> {
		self.0.model_aliases.as_ref()
	}

	pub fn merge(mut self, over: Config) -> Self {
		Arc::make_mut(&mut self.0).merge_with((*over.0).clone());
		self
	}

	pub fn get_model(&self, ref_name: &str) -> Result<String> {
		self.0.get_model(ref_name)
	}
}

// endregion: --- Config

// region:    --- ConfigInner

impl ConfigInner {
	pub fn from_toml_str(toml_str: &str) -> Result<Self> {
		let inner: Self = toml::from_str(toml_str)?;
		Ok(inner)
	}

	pub fn maestro_model(&self) -> &str {
		match self.maestro_model.as_deref() {
			Some(m) if !m.is_empty() => m,
			_ => "$small",
		}
	}

	pub fn workspace_working_dir(&self) -> Option<&SPath> {
		self.workspace_working_dir.as_ref()
	}

	pub fn merge_with(&mut self, over: ConfigInner) {
		if over.maestro_model.is_some() {
			self.maestro_model = over.maestro_model;
		}
		if over.workspace_working_dir.is_some() {
			self.workspace_working_dir = over.workspace_working_dir;
		}
		if let Some(over_sizes) = over.model_sizes {
			let sizes = self.model_sizes.get_or_insert_with(BTreeMap::new);
			sizes.extend(over_sizes);
		}
		if let Some(over_aliases) = over.model_aliases {
			let aliases = self.model_aliases.get_or_insert_with(BTreeMap::new);
			aliases.extend(over_aliases);
		}
	}

	pub fn get_model(&self, ref_name: &str) -> Result<String> {
		let trimmed = ref_name.trim();
		if trimmed.is_empty() {
			return Ok(String::new());
		}

		// 1. Check size preset if starts with '$'
		let target = if let Some(size_key) = trimmed.strip_prefix('$') {
			self.model_sizes
				.as_ref()
				.and_then(|sizes| sizes.get(size_key))
				.map(|s| s.as_str())
				.ok_or_else(|| Error::ModelSizeNotFound(trimmed.to_string()))?
		} else {
			trimmed
		};

		// 2. Resolve alias chain with cycle detection
		let mut current = target;
		let mut visited = HashSet::new();
		visited.insert(current);

		for _ in 0..MAX_ALIAS_DEPTH {
			if let Some(aliased) = self.model_aliases.as_ref().and_then(|aliases| aliases.get(current)) {
				let next = aliased.as_str();
				if visited.contains(next) {
					return Err(Error::ModelAliasCycle(format!(
						"Circular model alias detected for '{trimmed}' at '{next}'"
					)));
				}
				visited.insert(next);
				current = next;
			} else {
				return Ok(current.to_string());
			}
		}

		Err(Error::ModelAliasCycle(format!(
			"Model alias resolution exceeded max depth ({MAX_ALIAS_DEPTH}) for '{trimmed}'"
		)))
	}
}

// endregion: --- Config

// region:    --- Froms

impl From<Arc<ConfigInner>> for Config {
	fn from(inner: Arc<ConfigInner>) -> Self {
		Self(inner)
	}
}

impl From<ConfigInner> for Config {
	fn from(inner: ConfigInner) -> Self {
		Self(Arc::new(inner))
	}
}

impl From<ConfigToml> for ConfigInner {
	fn from(toml: ConfigToml) -> Self {
		let workspace_working_dir = toml.workspace.and_then(|w| w.working_dir).map(SPath::from);
		let maestro_model = toml.maestro.and_then(|m| m.model);
		Self {
			maestro_model,
			workspace_working_dir,
			model_sizes: toml.model_sizes,
			model_aliases: toml.model_aliases,
		}
	}
}

impl From<ConfigInner> for ConfigToml {
	fn from(inner: ConfigInner) -> Self {
		let workspace = inner.workspace_working_dir.map(|p| WorkspaceToml {
			working_dir: Some(p.as_str().to_string()),
		});
		let maestro = inner.maestro_model.map(|m| MaestroToml {
			model: Some(m),
		});
		Self {
			workspace,
			maestro,
			model_sizes: inner.model_sizes,
			model_aliases: inner.model_aliases,
		}
	}
}

// endregion: --- Froms

// region:    --- Support

#[derive(Debug, Serialize, Deserialize)]
struct ConfigToml {
	#[serde(default, skip_serializing_if = "Option::is_none")]
	workspace: Option<WorkspaceToml>,

	#[serde(default, skip_serializing_if = "Option::is_none")]
	maestro: Option<MaestroToml>,

	#[serde(default, skip_serializing_if = "Option::is_none")]
	model_sizes: Option<BTreeMap<String, String>>,

	#[serde(default, skip_serializing_if = "Option::is_none")]
	model_aliases: Option<BTreeMap<String, String>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct WorkspaceToml {
	#[serde(default, skip_serializing_if = "Option::is_none")]
	working_dir: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct MaestroToml {
	#[serde(default, skip_serializing_if = "Option::is_none")]
	model: Option<String>,
}

// endregion: --- Support

// region:    --- Tests

#[cfg(test)]
mod tests {
	type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>;

	use super::*;

	const SAMPLE_CONFIG: &str = r#"
[maestro]
model = "$small"

[model_sizes]
small   = "lite"
medium  = "flash"
big     = "sol"

[model_aliases]
lite31  = "gemini-3.1-flash-lite"
lite    = "gemini-3.5-flash-lite"
flash   = "gemini-3.7-flash"
sol     = "gpt-5.6-sol"
chain_a = "chain_b"
chain_b = "chain_c"
chain_c = "final-model"
loop_a  = "loop_b"
loop_b  = "loop_a"
"#;

	#[test]
	fn test_config_from_toml_str_defaults() -> Result<()> {
		// -- Exec
		let config = Config::from_toml_str("")?;

		// -- Check
		assert_eq!(config.maestro_model(), "$small");
		assert!(config.workspace_working_dir().is_none());
		assert!(config.model_sizes.is_none());
		assert!(config.model_aliases.is_none());

		Ok(())
	}

	#[test]
	fn test_config_fluid_builders() -> Result<()> {
		// -- Exec
		let config = Config::default()
			.with_maestro_model("custom-model")
			.with_workspace_working_dir("./sub-crate")
			.append_model_size("small", "fast-one")
			.append_model_alias("fast-one", "gpt-4o-mini");

		// -- Check
		assert_eq!(config.maestro_model(), "custom-model");
		assert_eq!(
			config.workspace_working_dir().map(|p| p.as_str()),
			Some("./sub-crate")
		);
		let resolved = config.get_model("$small")?;
		assert_eq!(resolved, "gpt-4o-mini");

		Ok(())
	}

	#[test]
	fn test_config_merge() -> Result<()> {
		// -- Setup & Fixtures
		let base = Config::from_toml_str(
			r#"
[maestro]
model = "$small"

[model_sizes]
small = "lite"
"#,
		)?;

		let over = Config::from_toml_str(
			r#"
[workspace]
working_dir = "crates/zc-core"

[model_sizes]
medium = "flash"
"#,
		)?;

		// -- Exec
		let merged = base.merge(over);

		// -- Check
		assert_eq!(merged.maestro_model(), "$small");
		assert_eq!(
			merged.workspace_working_dir().map(|p| p.as_str()),
			Some("crates/zc-core")
		);
		assert_eq!(
			merged.model_sizes().unwrap().get("small").map(|s| s.as_str()),
			Some("lite")
		);
		assert_eq!(
			merged.model_sizes().unwrap().get("medium").map(|s| s.as_str()),
			Some("flash")
		);

		Ok(())
	}

	#[test]
	fn test_config_get_model_size_preset() -> Result<()> {
		// -- Setup & Fixtures
		let config = Config::from_toml_str(SAMPLE_CONFIG)?;

		// -- Exec
		let model = config.get_model("$small")?;

		// -- Check
		assert_eq!(model, "gemini-3.5-flash-lite");

		Ok(())
	}

	#[test]
	fn test_config_get_model_direct_alias() -> Result<()> {
		// -- Setup & Fixtures
		let config = Config::from_toml_str(SAMPLE_CONFIG)?;

		// -- Exec
		let model = config.get_model("flash")?;

		// -- Check
		assert_eq!(model, "gemini-3.7-flash");

		Ok(())
	}

	#[test]
	fn test_config_get_model_raw_fallback() -> Result<()> {
		// -- Setup & Fixtures
		let config = Config::from_toml_str(SAMPLE_CONFIG)?;

		// -- Exec
		let model = config.get_model("custom/my-raw-model")?;

		// -- Check
		assert_eq!(model, "custom/my-raw-model");

		Ok(())
	}

	#[test]
	fn test_config_get_model_multi_hop_alias() -> Result<()> {
		// -- Setup & Fixtures
		let config = Config::from_toml_str(SAMPLE_CONFIG)?;

		// -- Exec
		let model = config.get_model("chain_a")?;

		// -- Check
		assert_eq!(model, "final-model");

		Ok(())
	}

	#[test]
	fn test_config_get_model_cycle_detection() -> Result<()> {
		// -- Setup & Fixtures
		let config = Config::from_toml_str(SAMPLE_CONFIG)?;

		// -- Exec
		let result = config.get_model("loop_a");

		// -- Check
		assert!(result.is_err());

		Ok(())
	}

	#[test]
	fn test_config_get_model_unknown_size() -> Result<()> {
		// -- Setup & Fixtures
		let config = Config::from_toml_str(SAMPLE_CONFIG)?;

		// -- Exec
		let result = config.get_model("$huge");

		// -- Check
		assert!(result.is_err());

		Ok(())
	}
}

// endregion: --- Tests
