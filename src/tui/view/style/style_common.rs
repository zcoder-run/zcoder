use crate::tui::view::style;
use ratatui::style::Style;

pub fn style_status(has_error: bool, waiting: bool) -> Style {
	if has_error {
		style::STL_STATUS_ERR
	} else if waiting {
		style::STL_STATUS_WAITING
	} else {
		style::STL_STATUS_READY
	}
}

pub fn style_input(waiting: bool) -> Style {
	if waiting {
		style::STL_INPUT_WAITING
	} else {
		style::STL_INPUT
	}
}
