use simple_fs::SPath;
use zc_core::exec::{ExecEventRx, Executor, ExecutorConfig};
use zc_router::{CoreMsgTx, new_core_msg_channel, run_router};

// region:    --- Config

/// Configuration for the in-process `zc-base` server.
#[derive(Debug, Clone)]
pub struct ZcBaseConfig {
	wspace_dir: SPath,
	base_dir: Option<SPath>,
	model: Option<String>,
}

impl Default for ZcBaseConfig {
	fn default() -> Self {
		let wspace_dir = simple_fs::current_dir().unwrap_or_else(|_| SPath::from("."));
		Self {
			wspace_dir,
			base_dir: None,
			model: None,
		}
	}
}

impl ZcBaseConfig {
	pub fn with_wspace_dir(mut self, wspace_dir: impl Into<SPath>) -> Self {
		self.wspace_dir = wspace_dir.into();
		self
	}

	pub fn with_base_dir(mut self, base_dir: impl Into<SPath>) -> Self {
		self.base_dir = Some(base_dir.into());
		self
	}

	pub fn with_model(mut self, model: impl Into<String>) -> Self {
		self.model = Some(model.into());
		self
	}

	fn into_executor_config(self) -> ExecutorConfig {
		let mut executor_config = ExecutorConfig::default().with_wspace_dir(self.wspace_dir);
		if let Some(base_dir) = self.base_dir {
			executor_config = executor_config.with_base_dir(base_dir);
		}
		if let Some(model) = self.model {
			executor_config = executor_config.with_model(model);
		}
		executor_config
	}
}

// endregion: --- Config

// region:    --- ZcBase

/// In-process `zc-base` server.
///
/// Owns Core initialization and the router loop. The transport is in-process MPSC
/// today; a wire transport can replace it later without changing the message contract.
pub struct ZcBase {
	core_msg_tx: CoreMsgTx,
	exec_event_rx: ExecEventRx,
}

impl ZcBase {
	/// Starts the in-process server.
	///
	/// Spawns the Core executor and the router loop, then returns the handles a frontend
	/// needs to reach Core. Must be called from within a Tokio runtime.
	pub fn start(config: ZcBaseConfig) -> zc_core::exec::Result<Self> {
		// -- Core initialization
		let (executor, exec_cmd_tx, exec_event_rx) = Executor::new(config.into_executor_config())?;
		tokio::spawn(async move { executor.start().await });

		// -- Router loop
		let (core_msg_tx, core_msg_rx) = new_core_msg_channel();
		tokio::spawn(async move {
			if let Err(err) = run_router(core_msg_rx, exec_cmd_tx).await {
				tracing::warn!("router loop ended with error: {err:?}");
			}
		});

		Ok(Self {
			core_msg_tx,
			exec_event_rx,
		})
	}

	/// Returns a Core message sender for a frontend (commands and requests).
	pub fn core_msg_tx(&self) -> CoreMsgTx {
		self.core_msg_tx.clone()
	}

	/// Returns the Core run lifecycle event receiver for a frontend.
	pub fn exec_event_rx(self) -> ExecEventRx {
		self.exec_event_rx
	}
}

// endregion: --- ZcBase
