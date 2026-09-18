//! Shared constants for the `zc` client and `zc base` server processes.
//!
//! This is the single source of truth for the socket path, the base home
//! directory names, the workspace marker, the debug log names, and the idle
//! grace period. `ZCODER_BASE_DIR` optionally overrides the base home directory.

pub const BASE_SOCK_PATH: &str = "/tmp/zcoder-base.sock";
pub const ZBASE_DIR_NAME: &str = "zcoder-base";
pub const ZCODER_BASE_DIR_ENV: &str = "ZCODER_BASE_DIR";
pub const CONFIG_DIR_NAME: &str = ".config";
pub const WKS_MARKER_DIR_NAME: &str = ".zcoder";
pub const DEBUG_LOG_DIR_NAME: &str = "debug-log";
pub const DEBUG_LOG_FILE_NAME: &str = "log.txt";
pub const BASE_IDLE_GRACE_SECS: u64 = 5;
