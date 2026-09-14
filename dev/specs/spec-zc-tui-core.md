# zc-tui Core Specification

## Intent

Define the core TUI runtime structure, app state, events, and event handling behavior for the interactive prompt loop.

This is the core slice of `dev/specs/spec-zc-tui.md`. Read that file first for the crate overview and runtime flow, and `dev/specs/spec-zc-tui-view.md` for rendering.

The core TUI supports:

- editable prompt input
- prompt submission
- quit commands
- waiting state while executor work is active
- answer and error display state
- executor status handling
- optional timed redraws and transient feedback
- a modular state processor and event handler structure that can grow like the AIPack TUI

The scope covers `src/tui/core/` and its submodules. It does not cover visual layout details, which are defined in `spec-tui-view.md`.

Core module shape:

```text
src/tui/core/
  mod.rs                  # core module registry and selected re-exports
  app_event_handlers.rs   # maps terminal events and actions to state changes
  ping_timer.rs           # optional timed redraws and transient feedback
  term_reader.rs          # terminal input reader task
  tui_impl.rs             # terminal setup, teardown, and task startup
  tui_loop.rs             # event loop: draw, receive one event, handle
  app_state/              # renderable app state and state transitions
    mod.rs
    app_state_base.rs
    state_processor.rs
  event/                  # app event boundary
    mod.rs
    app_event.rs
  types/                  # small shared TUI enums and aliases
    mod.rs
```

Core app state:

```rust
pub struct AppState {
	input: String,
	waiting: bool,
	status: String,
	last_answer: Option<String>,
	last_error: Option<String>,
}
```

Core app events:

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

## Code Design

`src/tui/core/mod.rs` is the core module registry. It registers runtime modules and exposes only intentional core types:

```rust
mod app_event_handlers;
mod event;
mod term_reader;
mod tui_loop;

mod app_state;
mod ping_timer;
mod tui_impl;

pub mod types;

pub use app_state::AppState;
pub use ping_timer::{PingTimerTx, start_ping_timer};
pub use tui_impl::{AppTx, ExitTx, start_tui};
pub use types::*;
```

`AppState` is the single source of truth for renderable UI state. In the full structure, `AppState` is a public wrapper and the mutable fields can live in an internal core struct when state complexity grows.

State responsibilities:

- `input`
  - stores the current prompt buffer
  - initialized from `initial_prompt` when provided
- `waiting`
  - indicates that a prompt is currently running
  - disables prompt submission while true
- `status`
  - stores the current status line text
- `last_answer`
  - stores the most recent successful executor answer
- `last_error`
  - stores the most recent executor error

`app_state/state_processor.rs` owns state transitions that are more than direct field setters. It should:

- mutate `AppState` through narrow methods
- keep derived state updates in one place
- store pending outbound executor or app actions when that becomes useful
- preserve selection and scroll state when list and detail views are added
- request redraws when state changes require another render pass

`event/app_event.rs` owns the app event boundary:

- `AppEvent`
  - wraps terminal input, semantic actions, executor statuses, ticks, and redraw requests
- `AppActionEvent`
  - represents user intent after raw terminal input is interpreted
- scroll and navigation enums
  - represent directions and page actions as typed values when scrolling or navigation is introduced

`app_event_handlers.rs` converts raw terminal events and high-level app actions into state changes or executor actions:

- only processes key press events unless repeat handling is explicitly needed
- converts `Ctrl-c`, `/q`, `Enter`, character input, backspace, navigation keys, and scroll keys into semantic actions
- keeps modifier handling explicit
- keeps executor-facing commands behind `ExecActionEvent`

`tui_loop.rs` owns the event handling loop. On each iteration it renders the current state, waits for one `AppEvent`, then applies the event.

Loop responsibilities:

- draw before handling each event
- receive all app events through one channel
- preserve ordered UI events
- debounce or coalesce high-frequency non-UI events when introduced
- treat redraw and tick events as low-priority signals
- send executor actions through `ExecutorTx`
- exit on `AppActionEvent::Quit`

`tui_impl.rs` owns the runtime setup:

- initialize the terminal
- clear the initial screen
- create typed app channel wrappers such as `AppTx`
- forward executor status events into the app event stream
- start terminal reader tasks
- start ping timer tasks only when timed refreshes are needed
- restore the terminal before returning

Terminal reader behavior:

- `term_reader.rs` should create one `EventStream` and loop until the stream ends or sending to the app event channel fails
- each loop should create a short delay, for example 200 ms, and fuse it with `FutureExt::fuse()`
- each loop should also call `reader.next()` and fuse that terminal event future
- `tokio::select!` should wait on the fused delay and fused event future
- the delay branch should intentionally do nothing and continue the loop
- the event branch should forward successful terminal events into the app event stream
- send failure should end the task because the UI loop has already exited or is shutting down
- terminal read errors should be handled without panicking

The target shape follows the AIPack terminal reader pattern:

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

Terminal key behavior:

- `Ctrl-c`
  - sends `AppActionEvent::Quit`
- `Enter`
  - sends `Quit` when trimmed input is `/q`
  - sends `RunPrompt` when input is non-empty and the app is not waiting
- `Backspace`
  - removes the last input character
- character input
  - appends the character to the prompt buffer

Action behavior:

- `Quit`
  - exits the UI loop
- `RunPrompt(prompt)`
  - clears the input
  - sets `waiting` to true
  - clears `last_error`
  - sends `ExecActionEvent::RunPrompt(prompt)` to the executor

Executor status behavior:

- `RunStart`
  - sets status to `Sending to AI...`
- `RunEnd`
  - sets `waiting` to false
  - sets status to `Idle`
- `RunResult(answer)`
  - stores the answer as `last_answer`
- `RunError(err)`
  - stores the error as `last_error`

## Design Considerations

The state model starts intentionally small and render-oriented. The view can derive all visual output from `AppState` without needing to know about executor channels or terminal events.

The UI loop renders before receiving the next event. This makes every applied event visible on the next loop iteration and keeps rendering deterministic.

The terminal reader uses the delay and fuse pattern to keep terminal input asynchronous without busy polling. The short delay gives the reader loop a periodic wake point even when no terminal input arrives, which is useful for cooperative runtime behavior and future shutdown hooks. Fusing both futures protects the select loop from polling a completed future after one branch resolves. This also matches the AIPack behavior that tolerates late terminal events during shutdown, such as a resize event arriving after quit on Windows.

Prompt submission is blocked while `waiting` is true to avoid overlapping executor requests from the same TUI session.

The prompt is copied from the state before dispatch and then cleared when the action is handled. This keeps the submitted prompt stable even if the input buffer later changes.

Errors are cleared when a new prompt run starts, while previous answers remain until a new answer arrives. This allows the content area to show the latest meaningful result unless an error is present.

The AIPack-like core layout keeps runtime setup, event handling, state mutation, and view rendering separate. This prevents the TUI loop from becoming a service layer and keeps executor work outside the UI boundary.

Typed events, typed action enums, and narrow state accessors make it safer to add navigation, scrolling, popups, config, install states, run details, and task details without leaking raw terminal events or executor internals across the codebase.
