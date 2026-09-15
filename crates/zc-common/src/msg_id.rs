// region:    --- MsgId

/// Protocol-level message identity.
///
/// `MsgId` identifies a message traveling through the messaging layer,
/// which is distinct from the domain/entity `Id` that identifies a
/// persisted application/database entity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
