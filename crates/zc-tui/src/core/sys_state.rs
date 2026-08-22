use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System, get_current_pid};

// region:    --- Types

#[allow(dead_code)]
#[derive(Debug)]
pub struct SysState {
	system: System,
	pid: Option<Pid>,
}

// endregion: --- Types

#[allow(dead_code)]
impl SysState {
	pub fn refresh_memory(&mut self) -> u64 {
		let Some(pid) = self.pid else {
			return 0;
		};

		self.system.refresh_processes_specifics(
			ProcessesToUpdate::Some(&[pid]),
			true,
			ProcessRefreshKind::nothing().with_memory(),
		);

		self.system.process(pid).map(|p| p.memory()).unwrap_or(0)
	}
}

// endregion: --- Inherent Methods

// region:    --- Trait Implementations

impl Default for SysState {
	fn default() -> Self {
		Self {
			system: System::new(),
			pid: get_current_pid().ok(),
		}
	}
}

// endregion: --- Trait Implementations

// region:    --- Tests

#[cfg(test)]
mod tests {
	type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>;

	use super::*;

	#[test]
	fn test_sys_state_refresh_memory() -> Result<()> {
		let mut sys_state = SysState::default();
		let mem = sys_state.refresh_memory();
		assert!(mem > 0);

		Ok(())
	}
}

// endregion: --- Tests
