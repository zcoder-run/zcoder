use serde::{Deserialize, Serialize};
use std::path::{Component, Path};

// region:    --- Types

/// Identity a client announces when it attaches to the base.
///
/// `wspace_dir` is the workspace root and the sole identity key. `label` is display
/// metadata for server logs and listings; it is derived from `wspace_dir` when not
/// set explicitly and never participates in the get-or-create lookup.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientInfo {
	pub wspace_dir: String,
	pub label: Option<String>,
}

// endregion: --- Types

// region:    --- ClientInfo Constructors

impl ClientInfo {
	/// Builds client info from a workspace directory, deriving the label from the
	/// last two path components (for example `dev/zcoder`).
	pub fn from_wspace_dir(wspace_dir: impl Into<String>) -> Self {
		let wspace_dir = wspace_dir.into();
		let label = label_from_dir(&wspace_dir);
		Self { wspace_dir, label }
	}

	/// Sets an explicit label, replacing the derived one.
	pub fn with_label(mut self, label: impl Into<String>) -> Self {
		self.label = Some(label.into());
		self
	}
}

// endregion: --- ClientInfo Constructors

// region:    --- Support

/// Derives the display label from the last two path components of a directory.
fn label_from_dir(wspace_dir: &str) -> Option<String> {
	let parts: Vec<String> = Path::new(wspace_dir)
		.components()
		.filter_map(|component| match component {
			Component::Normal(part) => Some(part.to_string_lossy().into_owned()),
			_ => None,
		})
		.collect();

	match parts.len() {
		0 => None,
		1 => Some(parts[0].clone()),
		_ => {
			let last = parts.len() - 1;
			Some(format!("{}/{}", parts[last - 1], parts[last]))
		}
	}
}

// endregion: --- Support

// region:    --- Tests

#[cfg(test)]
mod tests {
	type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>;

	use super::*;

	#[test]
	fn test_client_info_serde_roundtrip() -> Result<()> {
		// -- Setup & Fixtures
		let info = ClientInfo::from_wspace_dir("/home/dev/zcoder");

		// -- Exec
		let json = serde_json::to_string(&info)?;
		let back: ClientInfo = serde_json::from_str(&json)?;

		// -- Check
		assert_eq!(back.wspace_dir, "/home/dev/zcoder");
		assert_eq!(back.label.as_deref(), Some("dev/zcoder"));

		Ok(())
	}

	#[test]
	fn test_client_info_label_derivation() -> Result<()> {
		// -- Deep path
		let info = ClientInfo::from_wspace_dir("/home/dev/zcoder");
		assert_eq!(info.label.as_deref(), Some("dev/zcoder"));

		// -- Single component
		let info = ClientInfo::from_wspace_dir("zcoder");
		assert_eq!(info.label.as_deref(), Some("zcoder"));

		// -- Root path
		let info = ClientInfo::from_wspace_dir("/");
		assert_eq!(info.label, None);

		Ok(())
	}

	#[test]
	fn test_client_info_with_label_override() -> Result<()> {
		// -- Exec
		let info = ClientInfo::from_wspace_dir("/home/dev/zcoder").with_label("custom");

		// -- Check
		assert_eq!(info.wspace_dir, "/home/dev/zcoder");
		assert_eq!(info.label.as_deref(), Some("custom"));

		Ok(())
	}
}

// endregion: --- Tests
