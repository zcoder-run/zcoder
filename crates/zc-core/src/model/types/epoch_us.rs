use crate::ScalarStruct;
use crate::model::{Error, Result};
use macro_rules_attribute as mra;

#[mra::derive(Debug, ScalarStruct!)]
pub struct EpochUs(i64);

impl EpochUs {
	pub fn as_i64(&self) -> i64 {
		self.0
	}
}

// from &i64
impl From<&i64> for EpochUs {
	fn from(val: &i64) -> EpochUs {
		EpochUs(*val)
	}
}

impl TryFrom<String> for EpochUs {
	type Error = Error;
	fn try_from(val: String) -> Result<EpochUs> {
		let id = val
			.parse()
			.map_err(|err| format!("id should be a number was '{val}'.\nCause: {err}"))?;
		Ok(EpochUs(id))
	}
}
