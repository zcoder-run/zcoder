use derive_more::{Display, From};

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug, Display, From)]
pub enum Error {
	#[from(String, &String, &str)]
	Custom(String),

	// -- Externals
	#[from]
	Io(std::io::Error),

	#[from]
	Genai(genai::Error),

	#[from]
	Udiffx(udiffx::Error),

	#[from]
	SimpleFs(simple_fs::Error),

	#[from]
	Inquire(inquire::InquireError),
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
