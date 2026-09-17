use serde_json::Value;

// region:    --- JSON Merge

/// Merges `overlay` into `base`.
///
/// For JSON objects, keys are merged recursively.
/// For scalars and arrays, values in `overlay` completely replace those in `base`.
/// If `overlay` contains a key with `Value::Null`, it replaces the base value with `Null`.
/// Absent keys in `overlay` keep their existing values from `base`.
pub fn merge(base: Value, overlay: Value) -> Value {
	match (base, overlay) {
		(Value::Object(mut base_map), Value::Object(overlay_map)) => {
			for (key, overlay_val) in overlay_map {
				let merged_val = match base_map.remove(&key) {
					Some(base_val) => merge(base_val, overlay_val),
					None => overlay_val,
				};
				base_map.insert(key, merged_val);
			}
			Value::Object(base_map)
		}
		(_base, overlay) => overlay,
	}
}

// endregion: --- JSON Merge

// region:    --- Tests

#[cfg(test)]
mod tests {
	type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>;

	use super::*;
	use serde_json::json;

	#[test]
	fn test_jsons_merge_nested_object() -> Result<()> {
		// -- Setup & Fixtures
		let base = json!({
			"app": {
				"name": "zc",
				"port": 8080
			},
			"debug": true
		});
		let overlay = json!({
			"app": {
				"port": 9090,
				"env": "prod"
			}
		});

		// -- Exec
		let merged = merge(base, overlay);

		// -- Check
		assert_eq!(
			merged,
			json!({
				"app": {
					"name": "zc",
					"port": 9090,
					"env": "prod"
				},
				"debug": true
			})
		);
		Ok(())
	}

	#[test]
	fn test_jsons_merge_scalar_override() -> Result<()> {
		// -- Setup & Fixtures
		let base = json!({ "count": 10, "label": "old" });
		let overlay = json!({ "count": 20, "label": "new" });

		// -- Exec
		let merged = merge(base, overlay);

		// -- Check
		assert_eq!(merged, json!({ "count": 20, "label": "new" }));
		Ok(())
	}

	#[test]
	fn test_jsons_merge_array_full_replace() -> Result<()> {
		// -- Setup & Fixtures
		let base = json!({ "items": [1, 2, 3] });
		let overlay = json!({ "items": [4, 5] });

		// -- Exec
		let merged = merge(base, overlay);

		// -- Check
		assert_eq!(merged, json!({ "items": [4, 5] }));
		Ok(())
	}

	#[test]
	fn test_jsons_merge_overlay_null_handling() -> Result<()> {
		// -- Setup & Fixtures
		let base = json!({ "key": "exists", "keep": 42 });
		let overlay = json!({ "key": null });

		// -- Exec
		let merged = merge(base, overlay);

		// -- Check
		assert_eq!(merged, json!({ "key": null, "keep": 42 }));
		Ok(())
	}

	#[test]
	fn test_jsons_merge_absent_overlay_key_keeps_base() -> Result<()> {
		// -- Setup & Fixtures
		let base = json!({ "a": 1, "b": 2, "c": 3 });
		let overlay = json!({ "b": 20 });

		// -- Exec
		let merged = merge(base, overlay);

		// -- Check
		assert_eq!(merged, json!({ "a": 1, "b": 20, "c": 3 }));
		Ok(())
	}
}

// endregion: --- Tests
