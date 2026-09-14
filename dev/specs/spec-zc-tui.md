# zc-tui Specification

## Intent

Define the terminal UI lifecycle, module boundaries, runtime structure, app state, events, event handling, and rendering for the `zcoder` interactive CLI.

This is the entry spec for the crate. Read these next:

- `dev/specs/spec-zc-tui-core.md`: core runtime, events, app state, and event handling.

- `dev/specs/spec-zc-tui-view.md`: layout, section views, styles, and render helpers.

The TUI provides:

- terminal initialization and restoration
- a central app event channel
- terminal input forwarding
- executor status forwarding
- a UI loop that renders state and dispatches user actions
- editable prompt input and prompt submission
- quit commands
- a waiting state while executor work is active
- answer and error display state
- executor status handling
- optional timed redraws and transient feedback
- a modular state processor, event handler, and view structure that can grow like the AIPack TUI

The scope covers the top-level TUI module, the core runtime modules, shared support modules, and the view module registry. It does not define executor internals, AI provider behavior, or individual file-change application logic.

## Dependency and Public API

`zc-tui` depends on `zc-core` for the executor sender and executor status types, and on `zc-common` where it needs shared helpers.

The public entry point is the only exported TUI API:

```rust
pub async fn start_tui(
	executor_tx: ExecutorTx,
	status_rx: Receiver<ExecStatusEvent>,
	initial_prompt: Option<String>,
) -> Result<()>;
```

`crates/zc-tui/src/lib.rs` is the crate registry and public entry surface:

```rust
// region:    --- Modules

mod core;
mod error;
mod view;

pub use core::start_tui;
pub use error::{Error, Result};

// endregion: --- Modules
```

## Module Structure
The TUI is organized around a thin crate root plus three top-level areas:
The TUI is organized around three top-level areas:

```text
  lib.rs         # crate registry and public entry surface
  error.rs       # TUI error and result types
  core/          # runtime, events, app state, and event handling
  support/       # shared formatting and utility helpers
  view/          # rendering, layout, components, and styles
  view/
```
  lib.rs`: crate registry and public entry surface. It re-exports `start_tui`, `Error`, and `Result`.
  core/`: runtime setup, event handling, app state, and loop control.
  support/`: stateless formatting and utility helpers shared by `core` and `view`.
  view/`: pure render layer over the app state.

The detailed module layout for `core/` lives in `dev/specs/spec-zc-tui-core.md`, and for `view/` in `dev/specs/spec-zc-tui-view.md`.
- `view/`: renders `AppState`, owns layout composition, reusable components, style constants, and view facades

Event flow:

```text
Terminal input -> AppEvent::Term -> tui_loop
User intent -> AppEvent::Action -> app_event_handlers/state_processor -> tui_loop
Executor status -> AppEvent::Exec -> tui_loop
Timer tick -> AppEvent::Tick -> tui_loop
Redraw request -> AppEvent::DoRedraw -> tui_loop
```

## Core Runtime

### Entry Point (tui_impl)

`tui_impl.rs` owns the runtime setup:

- initialize the terminal
- clear the initial screen
- create typed app channel wrappers such as `AppTx`
- forward executor status events into the app event stream
- start terminal reader tasks
- start ping timer tasks only when timed refreshes are needed
- restore the terminal before returning

Terminal lifecycle:

- `ratatui::init()` is called before the UI loop starts.
- The terminal is cleared before the first render.
- Mouse capture is paired with TUI setup and teardown when mouse handling is enabled.
- `ratatui::restore()` is called after the UI loop exits.
- The result from the UI loop is returned after terminal restoration.

### UI Loop (tui_loop)

`tui_loop.rs` owns the event handling loop. On each iteration it renders the current state, waits for one `AppEvent`, then applies the event.

Loop responsibilities:

- draw before handling each event
- receive all app events through one channel
- preserve ordered UI events
- debounce or coalesce high-frequency non-UI events when introduced
- treat redraw and tick events as low-priority signals
- send executor actions through `ExecutorTx`
- exit on `AppActionEvent::Quit`

### Terminal Reader (term_reader)

`term_reader.rs` should create one `EventStream` and loop until the stream ends or sending to the app event channel fails.

- each loop should create a short delay, for example 200 ms, and fuse it with `FutureExt::fuse()`
- each loop should also call `reader.next()` and fuse that terminal event future
- `tokio::select!` should wait on the fused delay and fused event future
- the delay branch should intentionally do nothing and continue the loop
- the event branch should forward successful terminal events into the app event stream
- send failure should end the task because the UI loop has already exited or is shutting down
- terminal read errors should be handled without panicking

Target shape:

```rust
loop {
	let delay = Delay::new(Duration::from_millis(200)).fuse();
	let event = reader.next().fuse();

	select! {
		_ = delay => {  },
		maybe_event = event => {
			// forward terminal event or exit on channel close
		}
	};
}
```

### Ping Timer (ping_timer)

`ping_timer.rs` provides `PingTimerTx` and `start_ping_timer` for optional timed redraws and transient feedback. The timer tasks are started only when timed refreshes are needed.

## App State

### State Model

`AppState` is the single source of truth for renderable UI state. In the full structure, `AppState` is a public wrapper and the mutable fields can live in an internal core struct when state complexity grows.

```rust
pub struct AppState {
	input: String,
	waiting: bool,
	status: String,
	last_answer: Option<String>,
	last_error: Option<String>,
}
```

State responsibilities:

- `input`: stores the current prompt buffer, initialized from `initial_prompt` when provided
- `waiting`: indicates that a prompt is currently running and disables prompt submission while true
- `status`: stores the current status line text
- `last_answer`: stores the most recent successful executor answer
- `last_error`: stores the most recent executor error

The state model starts intentionally small and render-oriented. The view can derive all visual output from `AppState` without needing to know about executor channels or terminal events.

### State Processor (state_processor)

`app_state/state_processor.rs` owns state transitions that are more than direct field setters. It should:

- mutate `AppState` through narrow methods
- keep derived state updates in one place
- store pending outbound executor or app actions when that becomes useful
- preserve selection and scroll state when list and detail views are added
- request redraws when state changes require another render pass

## Events

### AppEvent and AppActionEvent

`event/app_event.rs` owns the app event boundary:

```rust
pub enum AppEvent {
	Term(Event),
	Action(AppActionEvent),
	Exec(ExecStatusEvent),
	Tick,
	DoRedraw,
}

pub enum AppActionEvent {
	Quit,
	RunPrompt(String),
}
```

- `AppEvent`: wraps terminal input, semantic actions, executor statuses, ticks, and redraw requests
- `AppActionEvent`: represents user intent after raw terminal input is interpreted
- scroll and navigation enums: represent directions and page actions as typed values when scrolling or navigation is introduced

### Event Handlers (app_event_handlers)

`app_event_handlers.rs` converts raw terminal events and high-level app actions into state changes or executor actions:

- only processes key press events unless repeat handling is explicitly needed
- converts `Ctrl-c`, `/q`, `Enter`, character input, backspace, navigation keys, and scroll keys into semantic actions
- keeps modifier handling explicit
- keeps executor-facing commands behind `ExecActionEvent`

### Key Behavior

- `Ctrl-c`: sends `AppActionEvent::Quit`
- `Enter`:
  - sends `Quit` when trimmed input is `/q`
  - sends `RunPrompt` when input is non-empty and the app is not waiting
- `Backspace`: removes the last input character
- character input: appends the character to the prompt buffer

### Action Behavior

- `Quit`: exits the UI loop
- `RunPrompt(prompt)`:
  - clears the input
  - sets `waiting` to true
  - clears `last_error`
  - sends `ExecActionEvent::RunPrompt(prompt)` to the executor

### Executor Status Behavior

- `RunStart`: sets status to `Sending to AI...`
- `RunEnd`: sets `waiting` to false and sets status to `Idle`
- `RunResult(answer)`: stores the answer as `last_answer`
- `RunError(err)`: stores the error as `last_error`

## View

### View Module Structure

The view renders `AppState` into a terminal frame using `ratatui`. The primary interface is:

```rust
pub fn render(f: &mut Frame, state: &AppState);
```

The view is a pure render layer that is split by concern:

- `main_view.rs`: owns the render entry point and the top-level layout, and delegates the answer, status, prompt, and footer sections.
- `answer_view.rs`, `status_view.rs`, `prompt_view.rs`, `footer_view.rs`: render one section each.
- `comp/`: reusable UI components such as icons.
- `style/`: color constants, `Style` constants, and derived style helpers.
- `support/`: view-local helpers for lines, `Rect` placement, and text segmentation.

The full view module layout lives in `dev/specs/spec-zc-tui-view.md`.

### Main View Layout

The prompt UI uses a vertical layout with four sections:

- content, flexible height
- status, fixed height of 2, with the first row left empty
- input, fixed height of 3
- footer, fixed height of 1

Layout constraints:

```rust
[
	Constraint::Min(0),
	Constraint::Length(2),
	Constraint::Length(3),
	Constraint::Length(1),
]
```

Layering order:

- render the base background
- render answer or error content
- render status
- render prompt input
- render footer

Content behavior:

- shows `Error: {err}` when `state.last_error()` exists
- otherwise shows `state.last_answer()` when available
- otherwise shows `No answer yet. Type a prompt and press Enter.`
- renders on the dark answer background without a border or title
- uses ratatui layout constraints to add 1 character of left, top, right, and bottom padding
- wraps text with trimming enabled

Status behavior:

- renders `Status: {state.status()}`
- leaves one empty row above the status text so the status section has visual breathing room
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

### Main View Source Pattern

```rust
use crate::tui::AppState;
use crate::tui::view::{AnswerView, FooterView, PromptView, StatusView, style};
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::widgets::Block;

pub fn render(f: &mut Frame, state: &AppState) {
	let chunks = Layout::default()
		.direction(Direction::Vertical)
		.constraints([
			Constraint::Min(0),    // Content
			Constraint::Length(2), // Status
			Constraint::Length(3), // Input
			Constraint::Length(1), // Footer
		])
		.split(f.area());

	// -- Background
	f.render_widget(Block::new().style(style::STL_BKG), f.area());

	// -- Content
	AnswerView::render(f, chunks[0], state);

	// -- Status
	StatusView::render(f, chunks[1], state);

	// -- Input
	PromptView::render(f, chunks[2], state);

	// -- Footer
	FooterView::render(f, chunks[3], state);
}
```

### Section Views

- `answer_view.rs` owns content text selection and answer block rendering. It renders `Error: {err}` when `state.last_error()` exists, otherwise renders `state.last_answer()` when available, otherwise renders `No answer yet. Type a prompt and press Enter.`.
- `status_view.rs` owns the status line widget and calls shared style helpers from `style` for error, waiting, and ready states.
- `prompt_view.rs` owns prompt input rendering and cursor placement. It uses the dim input style while waiting and the default input style when editable.
- `footer_view.rs` owns the footer key hints. It renders `[Enter] Send`, `[/q] Quit`, and `[Ctrl-c] Quit`.

### Components, Style, and Support

`comp/mod.rs`:

```rust
// region:    --- Modules

mod icons;

pub use icons::*;

// endregion: --- Modules
```

`comp/icons.rs`:

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

`style/mod.rs`:

```rust
// region:    --- Modules

mod style_common;
mod style_consts;

pub use style_common::*;
pub use style_consts::*;

// endregion: --- Modules
```

`style/style_consts.rs`:

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

`style/style_common.rs`:

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

`support/mod.rs`:

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

`support/line_helpers.rs`:

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

`support/rect_ext.rs`:

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

`support/text_helpers.rs`:

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

- The view is a pure render layer over `AppState`. This separation keeps visual decisions independent from input handling and executor communication.
- Keeping only `main_view.rs` as the view coordinator prevents the app from gaining unused run, task, install, config, popup, or facade modules before those features exist.
- Fixed heights are used for header, status, input, and footer so the content area can absorb terminal resizing and answer length variation.
- The content area prioritizes errors over answers because errors require immediate attention and explain failed prompt runs.
- The input area is dimmed while waiting to communicate that prompt submission is temporarily disabled by the core event logic.
- The footer keeps available key actions visible without requiring a separate help screen.
- The UI loop renders before receiving the next event. This makes every applied event visible on the next loop iteration and keeps rendering deterministic.
- The terminal reader uses the delay and fuse pattern to keep terminal input asynchronous without busy polling. The short delay gives the reader loop a periodic wake point even when no terminal input arrives, and fusing both futures protects the select loop from polling a completed future after one branch resolves.
- Prompt submission is blocked while `waiting` is true to avoid overlapping executor requests from the same TUI session.
- The prompt is copied from the state before dispatch and then cleared when the action is handled. This keeps the submitted prompt stable even if the input buffer later changes.
- Errors are cleared when a new prompt run starts, while previous answers remain until a new answer arrives.
- The AIPack-like core layout keeps runtime setup, event handling, state mutation, and view rendering separate. This prevents the TUI loop from becoming a service layer and keeps executor work outside the UI boundary.
- Typed events, typed action enums, and narrow state accessors make it safer to add navigation, scrolling, popups, config, install states, run details, and task details without leaking raw terminal events or executor internals across the codebase.
- The design uses one app event stream so the UI loop can process terminal input, internal actions, ticks, and executor updates in a consistent order.
- The executor boundary is preserved. The TUI loop does not perform long-running work, model requests, file loading, extraction, application, installs, or checks. It only sends typed executor actions and reacts to executor lifecycle events.
