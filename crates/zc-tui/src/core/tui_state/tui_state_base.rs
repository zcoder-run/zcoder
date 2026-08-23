use crate::core::sys_state::SysState;
use crate::core::types::{ScrollIden, ScrollZones};
use crate::view::tblock::AiWorkInfo;
use ratatui::layout::Rect;

pub struct TuiState {
	input: String,
	waiting: bool,
	status: String,
	last_prompt: Option<String>,
	last_answer: Option<String>,
	last_error: Option<String>,
	ai_work_info: Option<AiWorkInfo>,
	work_start_us: Option<i64>,
	scroll_zones: ScrollZones,
	show_sys_states: bool,
	sys_state: SysState,
	memory: u64,
	db_memory: u64,

	// Reserved for multi-pane focus tracking and keyboard scroll routing.
	#[allow(dead_code)]
	active_scroll_zone_iden: Option<ScrollIden>,
}

impl TuiState {
	pub fn new(initial_prompt: Option<String>) -> Self {
		Self {
			input: initial_prompt.unwrap_or_default(),
			waiting: false,
			status: "Idle".to_string(),
			last_prompt: None,
			last_answer: None,
			last_error: None,
			ai_work_info: None,
			work_start_us: None,
			scroll_zones: ScrollZones::default(),
			show_sys_states: false,
			sys_state: SysState::default(),
			memory: 0,
			db_memory: 0,
			active_scroll_zone_iden: None,
		}
	}

	pub fn input(&self) -> &str {
		&self.input
	}

	pub fn push_input(&mut self, c: char) {
		self.input.push(c);
	}

	pub fn pop_input(&mut self) {
		self.input.pop();
	}

	pub fn clear_input(&mut self) {
		self.input.clear();
	}

	pub fn is_waiting(&self) -> bool {
		self.waiting
	}

	pub fn set_waiting(&mut self, waiting: bool) {
		self.waiting = waiting;
	}

	pub fn status(&self) -> &str {
		&self.status
	}

	pub fn set_status(&mut self, status: String) {
		self.status = status;
	}

	#[allow(dead_code)]
	pub fn last_prompt(&self) -> Option<&str> {
		self.last_prompt.as_deref()
	}

	pub fn set_last_prompt(&mut self, prompt: Option<String>) {
		self.last_prompt = prompt;
	}

	pub fn last_answer(&self) -> Option<&str> {
		self.last_answer.as_deref()
	}

	pub fn set_last_answer(&mut self, answer: Option<String>) {
		self.last_answer = answer;
	}

	pub fn last_error(&self) -> Option<&str> {
		self.last_error.as_deref()
	}

	pub fn set_last_error(&mut self, error: Option<String>) {
		self.last_error = error;
	}

	#[allow(dead_code)]
	pub fn ai_work_info(&self) -> Option<&AiWorkInfo> {
		self.ai_work_info.as_ref()
	}

	pub fn ai_work_info_mut(&mut self) -> Option<&mut AiWorkInfo> {
		self.ai_work_info.as_mut()
	}

	pub fn set_ai_work_info(&mut self, info: Option<AiWorkInfo>) {
		self.ai_work_info = info;
	}

	pub fn work_start_us(&self) -> Option<i64> {
		self.work_start_us
	}

	pub fn set_work_start_us(&mut self, start_us: Option<i64>) {
		self.work_start_us = start_us;
	}

	pub fn update_elapsed_time(&mut self, now_us: i64) {
		if let Some(start_us) = self.work_start_us
			&& let Some(info) = self.ai_work_info.as_mut()
			&& info.is_running
		{
			let elapsed_us = (now_us - start_us).max(0);
			info.duration = Some(format_duration_us(elapsed_us));
		}
	}

	#[allow(dead_code)]
	pub fn show_sys_states(&self) -> bool {
		self.show_sys_states
	}

	#[allow(dead_code)]
	pub fn set_show_sys_states(&mut self, show: bool) {
		self.show_sys_states = show;
	}

	#[allow(dead_code)]
	pub fn toggle_show_sys_states(&mut self) {
		self.show_sys_states = !self.show_sys_states;
	}

	#[allow(dead_code)]
	pub fn memory(&self) -> u64 {
		self.memory
	}

	#[allow(dead_code)]
	pub fn refresh_sys_state(&mut self) {
		self.memory = self.sys_state.refresh_memory();
	}

	#[allow(dead_code)]
	pub fn memory_fmt(&self) -> String {
		format_memory(self.memory)
	}

	#[allow(dead_code)]
	pub fn db_memory(&self) -> u64 {
		self.db_memory
	}

	#[allow(dead_code)]
	pub fn set_db_memory(&mut self, db_memory: u64) {
		self.db_memory = db_memory;
	}

	#[allow(dead_code)]
	pub fn db_memory_fmt(&self) -> String {
		format_memory(self.db_memory)
	}

	pub fn scroll_zones(&self) -> &ScrollZones {
		&self.scroll_zones
	}

	// Reserved for direct mutable access to the scroll zone registry.
	#[allow(dead_code)]
	pub fn scroll_zones_mut(&mut self) -> &mut ScrollZones {
		&mut self.scroll_zones
	}

	// Reserved for multi-pane focus queries and keyboard scroll routing.
	#[allow(dead_code)]
	pub fn active_scroll_zone_iden(&self) -> Option<ScrollIden> {
		self.active_scroll_zone_iden
	}

	// Reserved for multi-pane focus switching.
	#[allow(dead_code)]
	pub fn set_active_scroll_zone_iden(&mut self, iden: Option<ScrollIden>) {
		self.active_scroll_zone_iden = iden;
	}

	pub fn set_scroll_area(&mut self, iden: ScrollIden, area: Rect) {
		let zone = self.scroll_zones.get_or_create_zone_mut(iden);
		zone.set_area(area);
	}

	// Reserved for inactive zone cleanup when views or tabs switch.
	#[allow(dead_code)]
	pub fn clear_scroll_area(&mut self, iden: ScrollIden) {
		if let Some(zone) = self.scroll_zones.get_zone_mut(&iden) {
			zone.clear_area();
		}
	}

	pub fn get_scroll(&self, iden: ScrollIden) -> u16 {
		self.scroll_zones.get_zone(&iden).and_then(|z| z.scroll()).unwrap_or_default()
	}

	pub fn set_scroll(&mut self, iden: ScrollIden, scroll: u16) {
		let zone = self.scroll_zones.get_or_create_zone_mut(iden);
		zone.set_scroll(scroll);
	}

	pub fn inc_scroll(&mut self, iden: ScrollIden, amount: u16) {
		let current = self.get_scroll(iden);
		self.set_scroll(iden, current.saturating_add(amount));
	}

	pub fn dec_scroll(&mut self, iden: ScrollIden, amount: u16) {
		let current = self.get_scroll(iden);
		self.set_scroll(iden, current.saturating_sub(amount));
	}

	pub fn clamp_scroll(&mut self, iden: ScrollIden, line_count: usize) -> u16 {
		let Some(scroll_zone) = self.scroll_zones.get_zone_mut(&iden) else {
			return 0;
		};
		let area_height = scroll_zone.area().map(|a| a.height).unwrap_or_default();
		let max_scroll = line_count.saturating_sub(area_height as usize) as u16;
		let scroll = scroll_zone.scroll().unwrap_or_default();
		if scroll > max_scroll {
			scroll_zone.set_scroll(max_scroll);
			max_scroll
		} else {
			scroll
		}
	}
}

// region:    --- Support

fn format_memory(bytes: u64) -> String {
	const KB: f64 = 1024.0;
	const MB: f64 = KB * 1024.0;
	const GB: f64 = MB * 1024.0;

	let bytes_f = bytes as f64;
	if bytes_f >= GB {
		format!("{:.2} GB", bytes_f / GB)
	} else if bytes_f >= MB {
		format!("{:.2} MB", bytes_f / MB)
	} else if bytes_f >= KB {
		format!("{:.2} KB", bytes_f / KB)
	} else {
		format!("{bytes} B")
	}
}

pub(crate) fn format_duration_us(us: i64) -> String {
	let ms = us / 1000;
	if ms < 1000 {
		format!("{ms}ms")
	} else if ms < 60_000 {
		let secs = ms / 1000;
		let rem_ms = ms % 1000;
		if rem_ms > 0 {
			format!("{secs}s {rem_ms}ms")
		} else {
			format!("{secs}s")
		}
	} else {
		let mins = ms / 60_000;
		let rem_secs = (ms % 60_000) / 1000;
		if rem_secs > 0 {
			format!("{mins}m {rem_secs}s")
		} else {
			format!("{mins}m")
		}
	}
}

// endregion: --- Support

// region:    --- Tests

#[cfg(test)]
mod tests {
	type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>;

	use super::*;

	#[test]
	fn test_core_tui_state_scroll_inc_dec() -> Result<()> {
		// -- Setup & Fixtures
		let mut state = TuiState::new(None);
		let iden = ScrollIden::AnswerContent;

		// -- Exec & Check
		assert_eq!(state.get_scroll(iden), 0);

		state.inc_scroll(iden, 10);
		assert_eq!(state.get_scroll(iden), 10);

		state.inc_scroll(iden, 5);
		assert_eq!(state.get_scroll(iden), 15);

		state.dec_scroll(iden, 8);
		assert_eq!(state.get_scroll(iden), 7);

		// Dec beyond zero saturates at 0
		state.dec_scroll(iden, 20);
		assert_eq!(state.get_scroll(iden), 0);

		Ok(())
	}

	#[test]
	fn test_core_tui_state_clamp_scroll_basic() -> Result<()> {
		// -- Setup & Fixtures
		let mut state = TuiState::new(None);
		let iden = ScrollIden::AnswerContent;
		state.set_scroll_area(iden, Rect::new(0, 0, 80, 20));

		// -- Exec & Check
		// Line count <= viewport height -> max scroll is 0
		state.set_scroll(iden, 5);
		let clamped = state.clamp_scroll(iden, 15);
		assert_eq!(clamped, 0);
		assert_eq!(state.get_scroll(iden), 0);

		// Line count 50, height 20 -> max scroll 30
		// Scroll within bounds remains unchanged
		state.set_scroll(iden, 10);
		let clamped = state.clamp_scroll(iden, 50);
		assert_eq!(clamped, 10);
		assert_eq!(state.get_scroll(iden), 10);

		// Scroll exceeding bounds is clamped to max_scroll
		state.set_scroll(iden, 45);
		let clamped = state.clamp_scroll(iden, 50);
		assert_eq!(clamped, 30);
		assert_eq!(state.get_scroll(iden), 30);

		Ok(())
	}

	#[test]
	fn test_core_tui_state_clamp_scroll_zero_height() -> Result<()> {
		// -- Setup & Fixtures
		let mut state = TuiState::new(None);
		let iden = ScrollIden::AnswerContent;
		state.set_scroll_area(iden, Rect::new(0, 0, 80, 0));

		// -- Exec & Check
		// Zero height viewport: max_scroll = line_count
		state.set_scroll(iden, 20);
		let clamped = state.clamp_scroll(iden, 25);
		assert_eq!(clamped, 20);

		state.set_scroll(iden, 40);
		let clamped = state.clamp_scroll(iden, 25);
		assert_eq!(clamped, 25);

		Ok(())
	}

	#[test]
	fn test_core_tui_state_clamp_scroll_resize_reduction() -> Result<()> {
		// -- Setup & Fixtures
		let mut state = TuiState::new(None);
		let iden = ScrollIden::AnswerContent;
		state.set_scroll_area(iden, Rect::new(0, 0, 80, 20));
		state.set_scroll(iden, 30);

		// Initially at max_scroll (50 - 20 = 30)
		assert_eq!(state.clamp_scroll(iden, 50), 30);

		// Viewport expands to height 35 -> max_scroll becomes 50 - 35 = 15
		state.set_scroll_area(iden, Rect::new(0, 0, 80, 35));
		let clamped = state.clamp_scroll(iden, 50);
		assert_eq!(clamped, 15);
		assert_eq!(state.get_scroll(iden), 15);

		Ok(())
	}

	#[test]
	fn test_core_tui_state_sys_state_toggle_and_refresh() -> Result<()> {
		let mut state = TuiState::new(None);

		assert!(!state.show_sys_states());
		state.toggle_show_sys_states();
		assert!(state.show_sys_states());
		state.toggle_show_sys_states();
		assert!(!state.show_sys_states());

		state.refresh_sys_state();
		assert!(state.memory() > 0);
		assert!(
			state.memory_fmt().ends_with("MB")
				|| state.memory_fmt().ends_with("KB")
				|| state.memory_fmt().ends_with("GB")
		);

		Ok(())
	}

	#[test]
	fn test_core_tui_state_format_memory() -> Result<()> {
		assert_eq!(format_memory(500), "500 B");
		assert_eq!(format_memory(1024), "1.00 KB");
		assert_eq!(format_memory(1024 * 1024), "1.00 MB");
		assert_eq!(format_memory(49_820_467), "47.51 MB");
		assert_eq!(format_memory(1024 * 1024 * 1024), "1.00 GB");

		Ok(())
	}

	#[test]
	fn test_core_tui_state_db_memory() -> Result<()> {
		let mut state = TuiState::new(None);

		assert_eq!(state.db_memory(), 0);
		assert_eq!(state.db_memory_fmt(), "0 B");

		state.set_db_memory(247_459);
		assert_eq!(state.db_memory(), 247_459);
		assert_eq!(state.db_memory_fmt(), "241.66 KB");

		Ok(())
	}

	#[test]
	fn test_core_tui_state_last_prompt() -> Result<()> {
		let mut state = TuiState::new(None);
		assert_eq!(state.last_prompt(), None);

		state.set_last_prompt(Some("What is Rust?".to_string()));
		assert_eq!(state.last_prompt(), Some("What is Rust?"));

		state.set_last_prompt(None);
		assert_eq!(state.last_prompt(), None);

		Ok(())
	}

	#[test]
	fn test_core_tui_state_ai_work_info_and_elapsed() -> Result<()> {
		let mut state = TuiState::new(None);
		assert_eq!(state.ai_work_info(), None);

		let info = AiWorkInfo::new(true).with_model("gemini-2.5-flash");
		state.set_ai_work_info(Some(info));
		state.set_work_start_us(Some(1_000_000));

		state.update_elapsed_time(6_194_000);
		let work_info = state.ai_work_info().ok_or("should have work info")?;
		assert_eq!(work_info.duration.as_deref(), Some("5s 194ms"));
		assert_eq!(work_info.model.as_deref(), Some("gemini-2.5-flash"));

		Ok(())
	}

	#[test]
	fn test_core_tui_state_format_duration_us() -> Result<()> {
		assert_eq!(format_duration_us(250_000), "250ms");
		assert_eq!(format_duration_us(5_000_000), "5s");
		assert_eq!(format_duration_us(5_194_000), "5s 194ms");
		assert_eq!(format_duration_us(60_000_000), "1m");
		assert_eq!(format_duration_us(103_000_000), "1m 43s");

		Ok(())
	}
}

// endregion: --- Tests
