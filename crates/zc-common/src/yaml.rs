use crate::Result;
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

// region:    --- YAML Serialization / Deserialization

/// Serializes any serde serializable type into a YAML string.
pub fn to_yaml_string<T: Serialize>(value: &T) -> Result<String> {
	yaml_serde::to_string(value).map_err(Into::into)
}

/// Convenience helper to serialize a `serde_json::Value` to a YAML string.
pub fn json_to_yaml_string(value: &Value) -> Result<String> {
	to_yaml_string(value)
}

/// Deserializes a YAML string into the target serde deserializable type.
pub fn from_yaml_str<T: DeserializeOwned>(content: &str) -> Result<T> {
	yaml_serde::from_str(content).map_err(Into::into)
}

// endregion: --- YAML Serialization / Deserialization

// region:    --- Tests

#[cfg(test)]
mod tests {
	type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>; // For tests.

	use super::*;
	use serde_json::json;

	#[test]
	fn test_yaml_to_yaml_string_simple() -> Result<()> {
		// -- Setup & Fixtures
		let val = json!({
			"name": "zc",
			"count": 42,
			"active": true
		});

		// -- Exec
		let yaml_str = to_yaml_string(&val)?;

		// -- Check
		assert!(yaml_str.contains("name: zc"));
		assert!(yaml_str.contains("count: 42"));
		assert!(yaml_str.contains("active: true"));

		Ok(())
	}

	#[test]
	fn test_yaml_json_to_yaml_string_list() -> Result<()> {
		// -- Setup & Fixtures
		let val = json!(["item1", "item2"]);

		// -- Exec
		let yaml_str = json_to_yaml_string(&val)?;

		// -- Check
		assert!(yaml_str.contains("- item1"));
		assert!(yaml_str.contains("- item2"));

		Ok(())
	}

	#[test]
	fn test_yaml_from_yaml_str_simple() -> Result<()> {
		// -- Setup & Fixtures
		let yaml_str = "title: Hello\ncount: 10\n";

		// -- Exec
		let val: Value = from_yaml_str(yaml_str)?;

		// -- Check
		let title = val.get("title").and_then(|v| v.as_str()).ok_or("should have title")?;
		let count = val.get("count").and_then(|v| v.as_i64()).ok_or("should have count")?;
		assert_eq!(title, "Hello");
		assert_eq!(count, 10);

		Ok(())
	}
}

// endregion: --- Tests
