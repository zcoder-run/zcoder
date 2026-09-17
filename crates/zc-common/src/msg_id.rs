// region:    --- MsgId

use serde::{Deserialize, Serialize};

/// Protocol-level message identity.
///
/// `MsgId` identifies a message traveling through the messaging layer,
/// which is distinct from the domain/entity `Id` that identifies a
/// persisted application/database entity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct MsgId(u64);

impl MsgId {
	pub const fn new(val: u64) -> Self {
		Self(val)
	}

	pub const fn as_u64(self) -> u64 {
		self.0
	}
}

impl From<u64> for MsgId {
	fn from(val: u64) -> Self {
		Self(val)
	}
}

impl From<MsgId> for u64 {
	fn from(val: MsgId) -> Self {
		val.0
	}
}

impl std::fmt::Display for MsgId {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.0)
	}
}

// endregion: --- MsgId

// region:    --- Tests

#[cfg(test)]
mod tests {
	use super::*;

	type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>;

	#[test]
	fn test_msg_id_serde_roundtrip() -> Result<()> {
		let msg_id = MsgId::new(42);
		let json = serde_json::to_string(&msg_id)?;
		let msg_id_de: MsgId = serde_json::from_str(&json)?;
		assert_eq!(msg_id, msg_id_de);
		Ok(())
	}
}

// endregion: --- Tests
