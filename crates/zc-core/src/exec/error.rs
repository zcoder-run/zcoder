use crate::model;
use derive_more::{Display, From};

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug, Display, From)]
#[display("{self:?}")]
pub enum Error {
	#[from(String, &String, &str)]
	Custom(String),

	#[from]
	Model(model::Error),

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

	Aiprog(String),
}

// region:    --- Froms

impl From<aiprog::Error> for Error {
	fn from(val: aiprog::Error) -> Self {
		Self::Aiprog(val.to_string())
	}
}

impl From<aiprog::EngineError> for Error {
	fn from(val: aiprog::EngineError) -> Self {
		Self::Aiprog(val.to_string())
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
