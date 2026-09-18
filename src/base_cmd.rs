use crate::Result;
use simple_fs::SPath;
use zc_base::{BaseParts, ZcBaseConfig, start_base_parts};
use zc_common::consts::{BASE_IDLE_GRACE_SECS, DEBUG_LOG_DIR_NAME, DEBUG_LOG_FILE_NAME};
use zc_common::dirs::zbase_dir;
use zc_router::transport::{is_live, socket_path, unlink_if_exists};
use zc_router::{ConnWatch, RouterServer};

pub async fn run_base_cmd() -> Result<()> {
	let zbase_dir = zbase_dir()?;
	simple_fs::ensure_dir(&zbase_dir)?;

	let _tracing_guard = init_base_tracing(&zbase_dir)?;

	let sock_path = socket_path();
	if is_live(&sock_path).await {
		eprintln!(
			"Error: a live zcoder base server is already running on {}",
			sock_path.as_str()
		);
		std::process::exit(1);
	}
	unlink_if_exists(&sock_path)?;

	let base_config = ZcBaseConfig::default();
	let BaseParts {
		exec_cmd_tx,
		model_rpc_cmd_tx,
		wks_resolver,
		model_change_rx,
		exec_event_rx,
	} = start_base_parts(base_config)?;

	let server = RouterServer::bind(
		&sock_path,
		exec_cmd_tx,
		model_rpc_cmd_tx,
		wks_resolver,
		model_change_rx,
		exec_event_rx,
	)
	.await?;

	let conn_watch = server.conn_watch();

	let server_task = tokio::spawn(async move {
		if let Err(err) = server.run().await {
			tracing::error!("server run error: {err}");
		}
	});

	tracing::info!("zc base server listening on {}", sock_path.as_str());

	tokio::select! {
		res = server_task => {
			if let Err(err) = res {
				tracing::error!("server task join error: {err}");
			}
		}
		_ = run_idle_monitor(conn_watch) => {
			tracing::info!("idle monitor triggered shutdown");
		}
		_ = tokio::signal::ctrl_c() => {
			tracing::info!("received SIGINT (Ctrl-C), shutting down");
		}
		_ = sigterm() => {
			tracing::info!("received SIGTERM, shutting down");
		}
	}

	let _ = unlink_if_exists(&sock_path);
	tracing::info!("zc base server stopped");

	Ok(())
}

fn init_base_tracing(zbase_dir: &SPath) -> Result<Option<tracing_appender::non_blocking::WorkerGuard>> {
	let log_dir = zbase_dir.join(DEBUG_LOG_DIR_NAME);
	simple_fs::ensure_dir(&log_dir)?;

	let file_appender = tracing_appender::rolling::never(log_dir.as_str(), DEBUG_LOG_FILE_NAME);
	let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

	tracing_subscriber::fmt()
		.with_writer(non_blocking)
		.with_env_filter(tracing_subscriber::EnvFilter::new(
			"zcoder=debug,zc_base=debug,zc_core=debug,zc_router=debug,aicost=debug",
		))
		.without_time()
		.with_ansi(false)
		.init();

	Ok(Some(guard))
}

async fn run_idle_monitor(mut conn_watch: ConnWatch) {
	loop {
		if conn_watch.count() == 0 {
			tracing::debug!(
				"->> 0 clients connected, starting idle grace timer ({}s)",
				BASE_IDLE_GRACE_SECS
			);
			tokio::select! {
				_ = tokio::time::sleep(std::time::Duration::from_secs(BASE_IDLE_GRACE_SECS)) => {
					if conn_watch.count() == 0 {
						tracing::debug!("->> idle grace period expired with 0 clients, exiting");
						break;
					}
				}
				_ = conn_watch.wait_for_nonzero() => {
					tracing::debug!("->> client connected, cancelling idle shutdown");
				}
			}
		} else {
			conn_watch.wait_for_zero().await;
		}
	}
}

#[cfg(unix)]
async fn sigterm() {
	use tokio::signal::unix::{SignalKind, signal};
	if let Ok(mut sig) = signal(SignalKind::terminate()) {
		sig.recv().await;
	} else {
		std::future::pending::<()>().await;
	}
}

#[cfg(not(unix))]
async fn sigterm() {
	std::future::pending::<()>().await;
}
