// region:    --- Modules

pub mod config;
pub mod exec;
mod exec_event;
pub mod model;
mod model_change;
mod model_rpc;
mod prompts;

use crate::exec::{Executor, ExecutorConfig};
// endregion: --- Modules
pub use config::{Config, ConfigManager};
use exec_event::run_exec_event_loop;
pub use model::Db;
use model_change::run_model_change_loop;
use model_rpc::run_model_rpc_handler;
use simple_fs::SPath;
use zc_core::exec::ExecEventRx;
use zc_router::{
	ModelChangeRx, RouterClient, RouterMsgRx, RouterMsgTx, new_exec_event_channel, new_model_change_channel,
	new_model_rpc_cmd_channel, new_router_msg_channel, run_router,
};

// region:    --- Config

/// Configuration for the in-process `zc-base` server.
#[derive(Debug, Clone)]
pub struct ZcBaseConfig {
	wks_dir: SPath,
	base_dir: Option<SPath>,
	model: Option<String>,
}

impl Default for ZcBaseConfig {
	fn default() -> Self {
		let wks_dir = simple_fs::current_dir().unwrap_or_else(|_| SPath::from("."));
		Self {
			wks_dir,
			base_dir: None,
			model: None,
		}
	}
}

impl ZcBaseConfig {
	pub fn with_wks_dir(mut self, wks_dir: impl Into<SPath>) -> Self {
		self.wks_dir = wks_dir.into();
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
		let mut executor_config = ExecutorConfig::default().with_wks_dir(self.wks_dir);
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

// region:    --- Base Core

/// Starts the base role: Core initialization and the router dispatch loop.
///
/// Shared by the future server entry (`ZcBase::start`) and the temporary
/// in-process stand-in (`InProcBase::start`) so both start the base role the
/// same way. Must be called from within a Tokio runtime.
fn start_base_core(config: ZcBaseConfig) -> crate::exec::Result<(RouterMsgTx, ExecEventRx, RouterMsgRx)> {
	// -- Core initialization
	let (executor, exec_cmd_tx, exec_event_rx) = Executor::new(config.into_executor_config())?;
	tokio::spawn(async move { executor.start().await });

	// -- Model RPC handler
	let (model_rpc_cmd_tx, model_rpc_cmd_rx) = new_model_rpc_cmd_channel();
	tokio::spawn(async move { run_model_rpc_handler(model_rpc_cmd_rx).await });

	// -- Router loop
	let (router_msg_tx, router_msg_rx) = new_router_msg_channel();
	let (reply_tx, reply_rx) = new_router_msg_channel();
	tokio::spawn(async move {
		if let Err(err) = run_router(router_msg_rx, exec_cmd_tx, model_rpc_cmd_tx, reply_tx).await {
			tracing::warn!("router loop ended with error: {err:?}");
		}
	});

	Ok((router_msg_tx, exec_event_rx, reply_rx))
}

// endregion: --- Base Core

// region:    --- InProcBase

/// Temporary in-process stand-in for the future `zc base` server.
///
/// The `zc` (TUI) path uses this until a separate `zc base` server process
/// exists. It owns what the base role owns: Core initialization (executor) and
/// the router dispatch loop. Keeping it labeled and owned as the base role,
/// rather than as TUI UI logic, means the later split removes code from one
/// place instead of untangling TUI code.
pub struct InProcBase {
	router_msg_tx: RouterMsgTx,
	model_change_rx: ModelChangeRx,
	exec_event_rx: RouterMsgRx,
	reply_rx: RouterMsgRx,
}

impl InProcBase {
	/// Starts the in-process base: Core initialization and the router loop.
	///
	/// Must be called from within a Tokio runtime.
	pub fn start(config: ZcBaseConfig) -> crate::exec::Result<Self> {
		let (router_msg_tx, exec_event_source_rx, reply_rx) = start_base_core(config)?;

		// -- Core-facing pump loops
		let (model_change_tx, model_change_rx) = new_model_change_channel();
		tokio::spawn(async move { run_model_change_loop(model_change_tx).await });

		let (exec_event_tx, exec_event_rx) = new_exec_event_channel();
		tokio::spawn(async move { run_exec_event_loop(exec_event_source_rx, exec_event_tx).await });

		Ok(Self {
			router_msg_tx,
			model_change_rx,
			exec_event_rx,
			reply_rx,
		})
	}

	/// Returns the in-process router client for a frontend.
	pub fn router_client(self) -> RouterClient {
		RouterClient::in_proc(
			self.router_msg_tx,
			self.model_change_rx,
			self.exec_event_rx,
			self.reply_rx,
		)
	}

	/// Returns a Core message sender for a frontend (commands and requests).
	pub fn router_msg_tx(&self) -> RouterMsgTx {
		self.router_msg_tx.clone()
	}

	/// Returns the Core event receivers (model change and run lifecycle) for a frontend.
	pub fn into_event_rx(self) -> (ModelChangeRx, RouterMsgRx, RouterMsgRx) {
		(self.model_change_rx, self.exec_event_rx, self.reply_rx)
	}
}

// endregion: --- InProcBase

// region:    --- ZcBase

/// The `zc-base` server.
///
/// The future server that the `zc base` command starts: it owns Core
/// initialization and the router loop for the single-worker topology. The `zc`
/// (TUI) path does not start it; it uses the in-process stand-in
/// ([`InProcBase`]) until the server process exists.
///
/// The transport is in-process MPSC today; a wire transport can replace it
/// later without changing the message contract.
pub struct ZcBase {
	router_msg_tx: RouterMsgTx,
	exec_event_rx: ExecEventRx,
	reply_rx: RouterMsgRx,
}

impl ZcBase {
	/// Starts the server.
	///
	/// Starts the base role (Core initialization and the router loop), then
	/// returns the handles a frontend needs to reach Core. Must be called from
	/// within a Tokio runtime.
	pub fn start(config: ZcBaseConfig) -> crate::exec::Result<Self> {
		let (router_msg_tx, exec_event_rx, reply_rx) = start_base_core(config)?;
		Ok(Self {
			router_msg_tx,
			exec_event_rx,
			reply_rx,
		})
	}

	/// Returns a Core message sender for a frontend (commands and requests).
	pub fn router_msg_tx(&self) -> RouterMsgTx {
		self.router_msg_tx.clone()
	}

	/// Returns the Core run lifecycle event receiver for a frontend.
	pub fn exec_event_rx(self) -> (ExecEventRx, RouterMsgRx) {
		(self.exec_event_rx, self.reply_rx)
	}
}

// endregion: --- ZcBase
