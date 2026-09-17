use derive_more::{Display, From};

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug, Display, From)]
#[display("{self:?}")]
pub enum Error {
	#[from(String, &String, &str)]
	Custom(String),

	#[from]
	Io(std::io::Error),

	#[from]
	EventBase(zc_common::event_base::EventBaseError),

	/// A frame that could not be serialized or deserialized with postcard.
	#[from]
	Postcard(postcard::Error),

	/// A frame whose payload length exceeds the accepted maximum.
	#[from(ignore)]
	FrameTooLarge {
		len: usize,
		max: u32,
	},
}

// region:    --- Custom

impl Error {
	pub fn custom(val: impl Into<String>) -> Self {
		Self::Custom(val.into())
	}

	pub fn custom_from_err(err: impl std::error::Error) -> Self {
		Self::Custom(err.to_string())
	}

	/// Returns the reason text without the variant wrapper, for a user-facing reply.
	///
	/// The `Display` impl renders the debug form on purpose, so logs keep the
	/// variant context; an attach reply wants only the reason.
	pub fn reason(&self) -> String {
		match self {
			Error::Custom(message) => message.clone(),
			other => other.to_string(),
		}
	}
}

// endregion: --- Custom

// region:    --- Error Boilerplate

impl std::error::Error for Error {}

// endregion: --- Error Boilerplate
