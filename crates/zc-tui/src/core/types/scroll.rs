#![allow(unused)]

use ratatui::layout::{Position, Rect};
use std::collections::HashMap;

// region:    --- Types

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScrollIden {
	AnswerContent,
}

#[derive(Debug, Default, Clone)]
pub struct ScrollZone {
	area: Option<Rect>,
	scroll: Option<u16>,
	is_bottom: bool,
}

#[derive(Debug, Default, Clone)]
pub struct ScrollZones {
	zones: HashMap<ScrollIden, ScrollZone>,
}

// endregion: --- Types

// region:    --- ScrollZone Implementations

impl ScrollZone {
	pub fn area(&self) -> Option<Rect> {
		self.area
	}

	pub fn set_area(&mut self, area: Rect) {
		self.area = Some(area);
	}

	pub fn clear_area(&mut self) {
		self.area = None;
	}

	pub fn scroll(&self) -> Option<u16> {
		self.scroll
	}

	pub fn set_scroll(&mut self, scroll: u16) {
		self.scroll = Some(scroll);
	}

	pub fn is_bottom(&self) -> bool {
		self.is_bottom
	}

	pub fn set_is_bottom(&mut self, is_bottom: bool) {
		self.is_bottom = is_bottom;
	}
}

// endregion: --- ScrollZone Implementations

// region:    --- ScrollZones Implementations

impl ScrollZones {
	pub fn get_zone(&self, iden: &ScrollIden) -> Option<&ScrollZone> {
		self.zones.get(iden)
	}

	pub fn get_zone_mut(&mut self, iden: &ScrollIden) -> Option<&mut ScrollZone> {
		self.zones.get_mut(iden)
	}

	pub fn get_or_create_zone_mut(&mut self, iden: ScrollIden) -> &mut ScrollZone {
		self.zones.entry(iden).or_default()
	}

	pub fn find_zone_for_pos(&self, position: impl Into<Position>) -> Option<ScrollIden> {
		let position = position.into();
		self.zones
			.iter()
			.find(|(_, zone)| zone.area().is_some_and(|area| area.contains(position)))
			.map(|(iden, _)| *iden)
	}
}

// endregion: --- ScrollZones Implementations

// region:    --- Tests

#[cfg(test)]
mod tests {
	type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>;

	use super::*;

	#[test]
	fn test_core_types_scroll_zone_area_and_scroll() -> Result<()> {
		// -- Setup & Fixtures
		let mut zone = ScrollZone::default();
		let rect = Rect::new(5, 5, 20, 10);

		// -- Exec & Check
		assert!(zone.area().is_none());
		assert!(zone.scroll().is_none());
		assert!(!zone.is_bottom());

		zone.set_area(rect);
		zone.set_scroll(15);
		zone.set_is_bottom(true);

		assert_eq!(zone.area(), Some(rect));
		assert_eq!(zone.scroll(), Some(15));
		assert!(zone.is_bottom());

		zone.clear_area();
		assert!(zone.area().is_none());

		Ok(())
	}

	#[test]
	fn test_core_types_scroll_find_zone_for_pos_basic() -> Result<()> {
		// -- Setup & Fixtures
		let mut zones = ScrollZones::default();
		let rect = Rect::new(10, 10, 40, 20);
		zones.get_or_create_zone_mut(ScrollIden::AnswerContent).set_area(rect);

		// -- Exec & Check: inside matching coordinates
		assert_eq!(
			zones.find_zone_for_pos(Position::new(10, 10)),
			Some(ScrollIden::AnswerContent)
		);
		assert_eq!(
			zones.find_zone_for_pos(Position::new(25, 15)),
			Some(ScrollIden::AnswerContent)
		);
		assert_eq!(
			zones.find_zone_for_pos(Position::new(49, 29)),
			Some(ScrollIden::AnswerContent)
		);

		// -- Exec & Check: outside non-matching coordinates
		assert_eq!(zones.find_zone_for_pos(Position::new(9, 10)), None);
		assert_eq!(zones.find_zone_for_pos(Position::new(10, 9)), None);
		assert_eq!(zones.find_zone_for_pos(Position::new(50, 30)), None);
		assert_eq!(zones.find_zone_for_pos(Position::new(0, 0)), None);

		// -- Exec & Check: cleared area returns None
		zones.get_or_create_zone_mut(ScrollIden::AnswerContent).clear_area();
		assert_eq!(zones.find_zone_for_pos(Position::new(25, 15)), None);

		Ok(())
	}
}

// endregion: --- Tests
