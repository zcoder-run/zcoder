use derive_more::{Display, From};

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug, Display, From)]
#[display("{_0}")]
pub enum Error {
	#[from(String, &String, &str)]
	Custom(String),

	#[display("Model size not found: '{_0}'")]
	ModelSizeNotFound(String),

	#[display("Model alias cycle: '{_0}'")]
	ModelAliasCycle(String),

	// -- Externals
	#[from]
	Toml(toml::de::Error),

	#[from]
	Io(std::io::Error),
}

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
