# TUI View Specification

## Intent

Define the visual structure, render behavior, and view module organization for the terminal UI.

The view renders `AppState` into a terminal frame using `ratatui`. It provides:

- a header
- an answer or error content area
- a status line
- a prompt input area
- a footer with key hints
- a compact set of shared view components, styles, and rendering helpers

The scope covers only this view structure:

- `src/tui/view/mod.rs`
- `src/tui/view/main_view.rs`
- `src/tui/view/comp/mod.rs`
- `src/tui/view/comp/icons.rs`
- `src/tui/view/style/mod.rs`
- `src/tui/view/style/style_common.rs`
- `src/tui/view/style/style_consts.rs`
- `src/tui/view/support/mod.rs`
- `src/tui/view/support/line_helpers.rs`
- `src/tui/view/support/rect_ext.rs`
- `src/tui/view/support/text_helpers.rs`

The Rust module folder names should remain singular, `style/` and `support/`, so they match normal Rust module naming.

Primary interface:

```rust
pub fn render(f: &mut Frame, state: &AppState);
```

## Code Design

The view module is organized as a minimal top-level registry plus focused supporting modules:

```text
src/tui/view/
  mod.rs
  main_view.rs
  comp/
    mod.rs
    icons.rs
  style/
    mod.rs
    style_common.rs
    style_consts.rs
  support/
    mod.rs
    line_helpers.rs
    rect_ext.rs
    text_helpers.rs
```

No other view modules should be created for this spec. `main_view.rs` owns the full screen layout and delegates only to reusable helpers from `comp/`, `style/`, and `support/`.

`src/tui/view/mod.rs` is the view module registry and public re-export surface:

```rust
// region:    --- Modules

mod main_view;
mod support;

pub use main_view::*;

pub mod comp;
pub mod style;

// endregion: --- Modules
```

Module responsibilities:

- `main_view.rs`
  - owns the render entry point
  - owns the top-level layout
  - renders the base background
  - renders header, content, status, input, and footer
  - uses shared style constants instead of inline colors where practical
  - may use small private helper functions for text selection and style selection
- `comp/mod.rs`
  - declares reusable UI component modules
  - re-exports component helpers
- `comp/icons.rs`
  - stores small icon helpers that return `Span<'static>`
  - keeps icon glyphs and icon styling centralized
  - should not read or mutate `AppState`
- `style/mod.rs`
  - declares style submodules
  - re-exports common style helpers and constants
- `style/style_consts.rs`
  - stores color and `Style` constants
  - keeps terminal color choices in one place
  - uses short prefixes such as `CLR_` for colors and `STL_` for styles
- `style/style_common.rs`
  - stores derived style helper functions when a style depends on state
  - keeps style decision logic separate from layout code
- `support/mod.rs`
  - declares view-local helper modules
  - re-exports support helpers for `main_view.rs`
- `support/line_helpers.rs`
  - stores helper functions for composing `Line<'static>` collections
- `support/rect_ext.rs`
  - stores `Rect` extension helpers for margins, row selection, shrinking, and placement
- `support/text_helpers.rs`
  - stores text segmentation helpers, including path segmentation
  - keeps parsing behavior isolated from widgets

## Main View Layout

The prompt UI uses a vertical layout with five sections:

- header, fixed height of 3
- content, flexible height
- status, fixed height of 1
- input, fixed height of 3
- footer, fixed height of 1

Layout constraints:

```rust
[
	Constraint::Length(3),
	Constraint::Min(0),
	Constraint::Length(1),
	Constraint::Length(3),
	Constraint::Length(1),
]
```

Layering order:

- render the base background
- render header
- render answer or error content
- render status
- render prompt input
- render footer

Header behavior:

- renders the application name `zcoder`
- uses a bordered block
- uses cyan foreground styling from `style`

Content behavior:

- shows `Error: {err}` when `state.last_error()` exists
- otherwise shows `state.last_answer()` when available
- otherwise shows `No answer yet. Type a prompt and press Enter.`
- uses a bordered block titled `AI Answer`
- wraps text with trimming enabled

Status behavior:

- renders `Status: {state.status()}`
- uses an error style when an error exists
- uses a waiting style while waiting
- uses a ready style when idle and no error exists

Input behavior:

- renders the current prompt buffer from `state.input()`
- uses a bordered block titled `Prompt (/q to quit)`
- uses a dim style while waiting
- uses the default input style when editable

Footer behavior:

- renders key hints for sending and quitting
- shows `[Enter] Send`, `[/q] Quit`, and `[Ctrl-c] Quit`
- may use icon helpers from `comp::icons` if icons are desired

## File Creation Patterns

### `src/tui/view/main_view.rs`

`main_view.rs` should keep layout orchestration in one file and push reusable decisions into `style` and `support`.

Recommended source pattern:

```rust
use crate::tui::AppState;
use crate::tui::view::style;
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

pub fn render(f: &mut Frame, state: &AppState) {
	let chunks = Layout::default()
		.direction(Direction::Vertical)
		.constraints([
			Constraint::Length(3), // Header
			Constraint::Min(0),    // Content
			Constraint::Length(1), // Status
			Constraint::Length(3), // Input
			Constraint::Length(1), // Footer
		])
		.split(f.area());

	// -- Background
	f.render_widget(Block::new().style(style::STL_BKG), f.area());

	// -- Header
	let header = Paragraph::new(" zcoder ")
		.block(Block::default().borders(Borders::ALL))
		.style(style::STL_HEADER);
	f.render_widget(header, chunks[0]);

	// -- Content
	let content = Paragraph::new(content_text(state))
		.block(Block::default().borders(Borders::ALL).title(" AI Answer "))
		.wrap(Wrap { trim: true });
	f.render_widget(content, chunks[1]);

	// -- Status
	let status = Paragraph::new(format!(" Status: {} ", state.status())).style(status_style(state));
	f.render_widget(status, chunks[2]);

	// -- Input
	let input = Paragraph::new(state.input())
		.block(Block::default().borders(Borders::ALL).title(" Prompt (/q to quit) "))
		.style(input_style(state));
	f.render_widget(input, chunks[3]);

	// -- Footer
	let footer = Paragraph::new(" [Enter] Send  |  [/q] Quit  |  [Ctrl-c] Quit ");
	f.render_widget(footer, chunks[4]);
}

fn content_text(state: &AppState) -> String {
	if let Some(err) = &state.last_error() {
		format!("Error: {err}")
	} else if let Some(ans) = &state.last_answer() {
		ans.to_string()
	} else {
		"No answer yet. Type a prompt and press Enter.".to_string()
	}
}

fn status_style(state: &AppState) -> ratatui::style::Style {
	if state.last_error().is_some() {
		style::STL_STATUS_ERR
	} else if state.is_waiting() {
		style::STL_STATUS_WAITING
	} else {
		style::STL_STATUS_READY
	}
}

fn input_style(state: &AppState) -> ratatui::style::Style {
	if state.is_waiting() {
		style::STL_INPUT_WAITING
	} else {
		style::STL_INPUT
	}
}
```

### `src/tui/view/comp/mod.rs`

Component modules should follow the small registry and re-export pattern:

```rust
// region:    --- Modules

mod icons;

pub use icons::*;

// endregion: --- Modules
```

### `src/tui/view/comp/icons.rs`

Icon helpers should be pure functions that return styled spans:

```rust
use crate::tui::view::style;
use ratatui::text::Span;

pub fn ico_ready() -> Span<'static> {
	Span::styled("✔", style::CLR_TXT_READY)
}

pub fn ico_waiting() -> Span<'static> {
	Span::styled("⏸", style::CLR_TXT_WAITING)
}

pub fn ico_running() -> Span<'static> {
	Span::styled("▶", style::CLR_TXT_RUNNING)
}

pub fn ico_error() -> Span<'static> {
	Span::styled("✘", style::CLR_TXT_ERR)
}
```

### `src/tui/view/style/mod.rs`

Style modules should centralize constants and helper functions:

```rust
// region:    --- Modules

mod style_common;
mod style_consts;

pub use style_common::*;
pub use style_consts::*;

// endregion: --- Modules
```

### `src/tui/view/style/style_consts.rs`

Style constants should use centralized names and reusable `Style` values:

```rust
use ratatui::style::{Color, Style};

pub const CLR_BKG_BLACK: Color = Color::Indexed(0);

pub const CLR_TXT_DEFAULT: Color = Color::Indexed(252);
pub const CLR_TXT_MUTED: Color = Color::Indexed(244);
pub const CLR_TXT_HEADER: Color = Color::Cyan;
pub const CLR_TXT_READY: Color = Color::Green;
pub const CLR_TXT_WAITING: Color = Color::Yellow;
pub const CLR_TXT_RUNNING: Color = Color::Cyan;
pub const CLR_TXT_ERR: Color = Color::Red;

pub const STL_BKG: Style = Style::new().bg(CLR_BKG_BLACK);
pub const STL_HEADER: Style = Style::new().fg(CLR_TXT_HEADER);
pub const STL_INPUT: Style = Style::new();
pub const STL_INPUT_WAITING: Style = Style::new().fg(CLR_TXT_MUTED);
pub const STL_STATUS_READY: Style = Style::new().fg(CLR_TXT_READY);
pub const STL_STATUS_WAITING: Style = Style::new().fg(CLR_TXT_WAITING);
pub const STL_STATUS_ERR: Style = Style::new().fg(CLR_TXT_ERR);
```

### `src/tui/view/style/style_common.rs`

Derived style helpers should live outside `main_view.rs` when the logic is reused:

```rust
use crate::tui::view::style;
use ratatui::style::Style;

pub fn style_text_active(active: bool) -> Style {
	if active {
		style::STL_STATUS_READY
	} else {
		style::STL_INPUT_WAITING
	}
}
```

### `src/tui/view/support/mod.rs`

Support helpers should follow the same registry and re-export pattern:

```rust
// region:    --- Modules

mod line_helpers;
mod rect_ext;
mod text_helpers;

pub use line_helpers::*;
pub use rect_ext::*;
pub use text_helpers::*;

// endregion: --- Modules
```

### `src/tui/view/support/line_helpers.rs`

Line helper pattern:

```rust
use ratatui::text::Line;

pub fn extend_lines(all_lines: &mut Vec<Line<'static>>, lines: Vec<Line<'static>>, end_with_empty_line: bool) {
	if lines.is_empty() {
		return;
	}
	all_lines.extend(lines);
	if end_with_empty_line {
		all_lines.push(Line::default());
	}
}
```

### `src/tui/view/support/rect_ext.rs`

Rect helpers should be implemented as a trait on `Rect`:

```rust
use ratatui::layout::Rect;

/// Convenient Ratatui Area/Rect utility functions
#[allow(unused)]
pub trait RectExt {
	fn x_margin(&self, margin: u16) -> Rect;
	fn x_h_margin(&self, h_margin: u16) -> Rect;
	fn x_v_margin(&self, v_margin: u16) -> Rect;
	fn x_move_down(&self, y: u16) -> Rect;
	fn x_shrink_from_top(&self, height_to_remove: u16) -> Rect;
	fn x_shrink_left(&self, width: u16) -> Rect;
	fn x_row(&self, row_num: u16) -> Rect;
	fn x_top_right(&self, width: u16, height: u16) -> Rect;
	fn x_bottom_right(&self, width: u16, height: u16) -> Rect;
	fn x_with_x(&self, x: u16) -> Rect;
	fn x_with_y(&self, y: u16) -> Rect;
	fn x_width(&self, width: u16) -> Rect;
	fn x_height(&self, height: u16) -> Rect;
}

impl RectExt for Rect {
	fn x_margin(&self, margin: u16) -> Rect {
		let x = (self.x + margin).min(self.x + self.width);
		let y = (self.y + margin).min(self.y + self.height);
		let width = self.width.saturating_sub(2 * margin);
		let height = self.height.saturating_sub(2 * margin);

		Rect { x, y, width, height }
	}

	fn x_h_margin(&self, h_margin: u16) -> Rect {
		let x = (self.x + h_margin).min(self.x + self.width);
		let width = self.width.saturating_sub(2 * h_margin);

		Rect {
			x,
			y: self.y,
			width,
			height: self.height,
		}
	}

	fn x_v_margin(&self, v_margin: u16) -> Rect {
		let y = (self.y + v_margin).min(self.y + self.height);
		let height = self.height.saturating_sub(2 * v_margin);

		Rect {
			x: self.x,
			y,
			width: self.width,
			height,
		}
	}

	fn x_shrink_from_top(&self, height_to_remove: u16) -> Rect {
		let new_height = self.height.saturating_sub(height_to_remove);
		Rect {
			x: self.x,
			y: self.y + height_to_remove,
			width: self.width,
			height: new_height,
		}
	}

	fn x_shrink_left(&self, width: u16) -> Rect {
		let new_width = self.width.saturating_sub(width);
		let x = self.x + width;
		Rect {
			x,
			y: self.y,
			width: new_width,
			height: self.height,
		}
	}

	fn x_move_down(&self, y_offset: u16) -> Rect {
		Rect {
			x: self.x,
			y: self.y + y_offset,
			width: self.width,
			height: self.height,
		}
	}

	fn x_row(&self, row_num: u16) -> Rect {
		Rect {
			x: self.x,
			y: self.y + row_num - 1,
			width: self.width,
			height: 1.min(self.height),
		}
	}

	fn x_bottom_right(&self, width: u16, height: u16) -> Rect {
		Rect {
			x: self.x + self.width - width,
			y: self.y + self.height - height,
			width,
			height,
		}
	}

	fn x_top_right(&self, width: u16, height: u16) -> Rect {
		Rect {
			x: self.x + self.width - width,
			y: self.y,
			width,
			height,
		}
	}

	fn x_with_x(&self, x: u16) -> Rect {
		Rect {
			x,
			y: self.y,
			width: self.width,
			height: self.height,
		}
	}

	fn x_with_y(&self, y: u16) -> Rect {
		Rect {
			x: self.x,
			y,
			width: self.width,
			height: self.height,
		}
	}

	fn x_width(&self, width: u16) -> Rect {
		Rect {
			x: self.x,
			y: self.y,
			width,
			height: self.height,
		}
	}

	fn x_height(&self, height: u16) -> Rect {
		Rect {
			x: self.x,
			y: self.y,
			width: self.width,
			height,
		}
	}
}
```

### `src/tui/view/support/text_helpers.rs`

Text helper pattern:

```rust
use regex::Regex;
use std::sync::LazyLock;

pub struct TextSeg<'a> {
	pub text: String,
	pub file_path: Option<&'a str>,
}

pub fn segment_line_path(line: &str) -> Vec<TextSeg<'_>> {
	static RE: LazyLock<Regex> = LazyLock::new(|| {
		Regex::new(
			r#"(?x)
			~?[a-zA-Z0-9_@\-\./]+/[a-zA-Z0-9_@\-\.]+\.[a-zA-Z0-9]{2,5}
			|
			[a-zA-Z0-9_@\-]+(?:\.[a-zA-Z0-9_@\-]+)*\.[a-zA-Z][a-zA-Z0-9]{0,4}
			|
			\.[a-zA-Z][a-zA-Z0-9_\-]*(?:\.[a-zA-Z][a-zA-Z0-9]*)*
		"#,
		)
		.expect("Failed to compile segment_line_path regex")
	});

	let re = &*RE;
	let mut segments = Vec::new();
	let mut last_idx = 0;

	for m in re.find_iter(line) {
		let start = m.start();
		let end = m.end();
		let text = &line[start..end];

		if !text.contains('/') && !text.starts_with('.') {
			let next_byte = line.as_bytes().get(end).copied();
			if let Some(b) = next_byte
				&& (b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'.')
			{
				continue;
			}
		}

		if start > last_idx {
			segments.push(TextSeg {
				text: line[last_idx..start].to_string(),
				file_path: None,
			});
		}
		segments.push(TextSeg {
			text: text.to_string(),
			file_path: Some(text),
		});
		last_idx = end;
	}

	if last_idx < line.len() {
		segments.push(TextSeg {
			text: line[last_idx..].to_string(),
			file_path: None,
		});
	}

	if segments.is_empty() && !line.is_empty() {
		segments.push(TextSeg {
			text: line.to_string(),
			file_path: None,
		});
	}

	segments
}
```

## Design Considerations

The view is a pure render layer over `AppState`. This separation keeps visual decisions independent from input handling and executor communication.

Keeping only `main_view.rs` as the view coordinator prevents the app from gaining unused run, task, install, config, popup, or facade modules before those features exist.

Fixed heights are used for header, status, input, and footer so the content area can absorb terminal resizing and answer length variation.

The content area prioritizes errors over answers because errors require immediate attention and explain failed prompt runs.

The input area is dimmed while waiting to communicate that prompt submission is temporarily disabled by the core event logic.

The footer keeps available key actions visible without requiring a separate help screen.

Reusable components, style helpers, and support utilities are kept in separate folders so the single main view can stay focused on layout and rendering flow.
