use zc_core::model::{Id, ListAirOptions, ListRunOptions};

// region:    --- Types

/// RPC-style commands directed at Core model operations.
#[derive(Debug)]
pub enum ModelRpcCmd {
	// -- Run

	RunGet {
		id: Id,
	},

	RunList {
		options: ListRunOptions,
	},

	// -- Air

	AirGet {
		id: Id,
	},

	AirList {
		options: ListAirOptions,
	},
}

// endregion: --- Types
