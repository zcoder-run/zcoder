use crate::core::types::{ScrollIden, ScrollZones};
use ratatui::layout::Rect;

pub struct TuiState {
	input: String,
	waiting: bool,
	status: String,
	last_answer: Option<String>,
	last_error: Option<String>,
	scroll_zones: ScrollZones,

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
			last_answer: None,
			last_error: None,
			scroll_zones: ScrollZones::default(),
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
}

// endregion: --- Tests
