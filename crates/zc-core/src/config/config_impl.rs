use crate::config::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::ops::Deref;
use std::sync::Arc;

const MAX_ALIAS_DEPTH: usize = 16;

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
pub struct ConfigInner {
	#[serde(default)]
	pub maestro: MaestroConfig,

	#[serde(default)]
	pub model_sizes: HashMap<String, String>,

	#[serde(default)]
	pub model_aliases: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaestroConfig {
	#[serde(default = "default_maestro_model")]
	pub model: String,
}

fn default_maestro_model() -> String {
	"$small".to_string()
}

impl Default for MaestroConfig {
	fn default() -> Self {
		Self {
			model: default_maestro_model(),
		}
	}
}

// endregion: --- Types

// region:    --- Config

impl Config {
	pub fn from_toml_str(toml_str: &str) -> Result<Self> {
		let inner = ConfigInner::from_toml_str(toml_str)?;
		Ok(Self(Arc::new(inner)))
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

	pub fn get_model(&self, ref_name: &str) -> Result<String> {
		let trimmed = ref_name.trim();
		if trimmed.is_empty() {
			return Ok(String::new());
		}

		// 1. Check size preset if starts with '$'
		let target = if let Some(size_key) = trimmed.strip_prefix('$') {
			self.model_sizes
				.get(size_key)
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
			if let Some(aliased) = self.model_aliases.get(current) {
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

// endregion: --- Froms

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
		assert_eq!(config.maestro.model, "$small");
		assert!(config.model_sizes.is_empty());
		assert!(config.model_aliases.is_empty());

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
