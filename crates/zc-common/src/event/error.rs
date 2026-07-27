use derive_more::Display;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Display)]
#[display("{self:?}")]
pub enum Error {
	InvalidCapacity(usize),
	Tx(String),
	Rx(String),
}

impl std::error::Error for Error {}
