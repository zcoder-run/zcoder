use crate::{config, model, prompts};
use derive_more::{Display, From};

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug, Display, From)]
#[display("{_0}")]
pub enum Error {
	#[from(String, &String, &str)]
	Custom(String),

	// -- Modules
	#[from]
	Config(config::Error),

	#[from]
	Model(model::Error),

	#[from]
	Prompts(prompts::Error),

	// -- zc_common
	Tx(String),
	Rx(String),
	#[from]
	Common(zc_common::Error),
	#[from]
	Event(zc_common::event_base::EventBaseError),

	// -- External
	#[from]
	Genai(genai::Error),

	#[from]
	Udiffx(udiffx::Error),

	#[from]
	SimpleFs(simple_fs::Error),

	#[from]
	Asset(zc_asset::Error),

	Aiprog(String),
}

// region:    --- Froms

impl From<&aiprog::Error> for Error {
	fn from(val: &aiprog::Error) -> Self {
		match val {
			aiprog::Error::LuaScript(details) => {
				let mut msg = if let Some(line) = details.line_number() {
					format!("Lua Error (line {line}): {}", details.message())
				} else {
					format!("Lua Error: {}", details.message())
				};
				if let Some(surround) = details.surround_code() {
					msg.push_str(&format!("\n```lua\n{surround}\n```"));
				}
				if let Some(stack) = details.stack_trace() {
					msg.push_str(&format!("\nStack Trace:\n{stack}"));
				}
				Self::Aiprog(msg)
			}
			aiprog::Error::Engine(engine_err) => Self::from(engine_err),
			aiprog::Error::Custom(msg) => Self::Aiprog(msg.clone()),
			aiprog::Error::CustomAndCause(ctx, cause) => Self::Aiprog(format!("{ctx}: {cause}")),
			other => Self::Aiprog(other.to_string()),
		}
	}
}

impl From<aiprog::Error> for Error {
	fn from(val: aiprog::Error) -> Self {
		Self::from(&val)
	}
}

impl From<&aiprog::EngineError> for Error {
	fn from(val: &aiprog::EngineError) -> Self {
		match val {
			aiprog::EngineError::Custom(msg) => Self::Aiprog(msg.clone()),
			other => Self::Aiprog(other.to_string()),
		}
	}
}

impl From<aiprog::EngineError> for Error {
	fn from(val: aiprog::EngineError) -> Self {
		Self::from(&val)
	}
}

// endregion: --- Froms

// region:    --- Custom

impl Error {
	pub fn custom(val: impl Into<String>) -> Self {
		Self::Custom(val.into())
	}

	pub fn custom_from_err(err: impl std::error::Error) -> Self {
		Self::Custom(err.to_string())
	}
}

// endregion: --- Custom

// region:    --- Error Boilerplate

impl std::error::Error for Error {}

// endregion: --- Error Boilerplate

// region:    --- Tests

#[cfg(test)]
mod tests {
	type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>;

	use super::*;

	#[test]
	fn test_exec_error_from_aiprog_engine_error_custom() -> Result<()> {
		// -- Setup & Fixtures
		let raw_msg = "runtime error: script:5: attempt to index\nstack traceback:\n\tscript:5: in main";
		let engine_err = aiprog::EngineError::Custom(raw_msg.to_string());

		// -- Exec
		let exec_err = Error::from(engine_err);

		// -- Check
		assert_eq!(exec_err.to_string(), raw_msg);

		Ok(())
	}

	#[test]
	fn test_exec_error_from_aiprog_error_engine() -> Result<()> {
		// -- Setup & Fixtures
		let raw_msg = "custom engine error message\nline 2";
		let aiprog_err = aiprog::Error::Engine(aiprog::EngineError::Custom(raw_msg.to_string()));

		// -- Exec
		let exec_err = Error::from(aiprog_err);

		// -- Check
		assert_eq!(exec_err.to_string(), raw_msg);

		Ok(())
	}

	#[test]
	fn test_exec_error_from_aiprog_error_custom_and_cause() -> Result<()> {
		// -- Setup & Fixtures
		let aiprog_err = aiprog::Error::CustomAndCause("operation failed".to_string(), "file missing".to_string());

		// -- Exec
		let exec_err = Error::from(aiprog_err);

		// -- Check
		assert_eq!(exec_err.to_string(), "operation failed: file missing");

		Ok(())
	}

	#[test]
	fn test_exec_error_from_aiprog_error_custom() -> Result<()> {
		// -- Setup & Fixtures
		let aiprog_err = aiprog::Error::Custom("direct error".to_string());

		// -- Exec
		let exec_err = Error::from(aiprog_err);

		// -- Check
		assert_eq!(exec_err.to_string(), "direct error");

		Ok(())
	}
}

// endregion: --- Tests
