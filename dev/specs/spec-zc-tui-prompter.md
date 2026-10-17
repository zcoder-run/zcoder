# zc-tui Prompter Specification

## Intent

Describe the multi-line prompter used by the `zc-tui` prompt input, using the names from the code.

This spec covers the prompt buffer, key mapping, submit flow, wrap and cursor rules, clipboard use, and terminal setup. It builds on the TUI specs:

- `dev/specs/spec-zc-tui.md`: crate overview and runtime flow.
- `dev/specs/spec-zc-tui-core.md`: core runtime, events, and state model.
- `dev/specs/spec-zc-tui-view.md`: layout, section views, and styles.

## Scope

- A multi-line prompt with up to `PROMPTER_MAX_ROWS` (3) visible text rows.
- Cursor movement, selection, copy, and paste.
- An atomic submit at key time, with the text restored when the send fails.
- Pure modules for wrapping, key mapping, and view building, each with unit tests.
- Terminal setup that always restores mouse capture and bracketed paste.

Out of scope for now:

- Prompt history.
- `#label` highlighting.
- Click-to-copy hit regions.
- OSC 52 fallback for SSH sessions.
- Word-aware wrapping.
- Word delete with Ctrl-Backspace or Alt-Backspace.
- `KeyEventKind::Repeat` handling.

## Module Layout

```text
crates/zc-tui/src/core/
  prompter_keys.rs              # PrompterAction, PrompterMotion, is_send_key, map_prompter_key
  term_setup.rs                 # TermSetup: terminal init and teardown, Kitty and Alacritty detection
  tui_state/
    prompter_layout.rs          # VisualRow, wrap_rows, cursor_row_col, offset_for_row_col
    prompter_state.rs           # PrompterState: edits, movement, selection, submit, scroll
    prompter_state_tests.rs     # tests for PrompterState
    state_processor.rs          # PromptSubmit, submit_prompt, copy and paste support
  tui_event_handlers.rs         # maps terminal events to prompter actions and submits

crates/zc-tui/src/view/
  comp/prompter.rs              # PrompterRender, build_prompter_lines
  prompt_view.rs                # bordered block, title, height, cursor placement

crates/zc-common/src/clipboard.rs  # Clipboard wrapper over arboard
```

Responsibilities:

- `prompter_layout` is pure. It maps text to visual rows as byte ranges and resolves cursor and offset positions.
- `PrompterState` is the single source of truth for the prompt text. It owns the cursor, anchor, preferred column, scroll, and viewport width.
- `prompter_keys` is pure. It converts a `KeyEvent` into a `PrompterAction`, or `None`.
- `build_prompter_lines` returns only the visible rows, the selection spans, and the cursor position.
- `PromptView` owns the bordered block, the title, the height, and the cursor placement.

## Prompter State

```rust
pub const PROMPTER_MAX_ROWS: u16 = 3;

pub struct PrompterState {
	text: String,
	cursor: usize,
	anchor: Option<usize>,
	preferred_col: Option<u16>,
	scroll_y: u16,
	viewport_width: u16,
}
```

- `TuiState` owns the `PrompterState` through `prompter()` and `prompter_mut()`.
- `from_text` places the cursor at the end of the text. The CLI initial prompt uses it, and the prompt is never sent automatically.
- Edits replace the selection first, then reset `preferred_col` and clear `anchor`.
- `insert_str` normalizes `\r\n` and `\r` to `\n`, and tabs to 4 spaces.
- `backspace` and `delete` work on graphemes.
- `take_text` returns the text and resets every field except `viewport_width`.
- `restore_text` puts restored text before the current text, separated by a newline.

## Key Map

Movement keys with SHIFT extend the selection. Without SHIFT, they clear it.

| Key | Action |
| --- | --- |
| Printable key, no modifier or SHIFT | Insert, replacing any selection |
| Enter, Ctrl-Enter | Send |
| `\` then Enter | Newline, replacing the `\` |
| Shift-Enter, Alt-Enter (Option-Enter on macOS) | Newline, when the terminal reports them |
| Ctrl-j | Newline, on every terminal |
| Left, Right | Move one grapheme; collapse a selection to its start or end |
| Ctrl or Alt with Left, Right | Move by word |
| Alt-b, Alt-f | Move by word (macOS terminals) |
| Up, Down | Move one visual row, keeping the preferred column |
| Ctrl or Alt with Up, Down | Go to the text start or end |
| Home, End | Go to the start or end of the visual row |
| Ctrl-a | Select all |
| Esc | Clear the selection |
| Backspace, Delete | Remove the selection, or the grapheme before or at the cursor |
| Ctrl-v | Paste from the system clipboard |
| Alt-c | Copy the selection, or the whole prompt when nothing is selected |
| Ctrl-Home, Ctrl-End | Scroll the answer to the top or bottom |
| PageUp, PageDown | Scroll the answer |
| F2 | Toggle sys states |
| Ctrl-c | Quit |

Other rules:

- Tab is ignored.
- Bracketed paste (`Event::Paste`) inserts text and keeps its newlines.
- The mouse wheel scrolls the answer only. Clicks and drags in the prompt do nothing yet.

## Submit Flow

Submit happens at key time through `StateProcessor::submit_prompt`, which returns `PromptSubmit`:

```rust
pub enum PromptSubmit {
	Ignored,
	Newline,
	Quit,
	Run(String),
}
```

- `Newline`: the char before the cursor is `\` and nothing is selected. The `\` is replaced by a newline, and nothing is sent. This is checked first, so it works during a run.
- `Quit`: the trimmed text is exactly `/q`. Quitting is allowed during a run.
- `Ignored`: the trimmed text is empty, or a run is active. The buffer is unchanged.
- `Run(text)`: the trimmed text is sent with inner newlines kept. The buffer is cleared, `last_prompt` is set, waiting is set, and the last error is cleared.

The handler then queues `AppActionEvent::RunPrompt(text)`, which only sends to the router.

Send failure:

- The handler does not use `?` on this path. It logs with `tracing::warn!`.
- It calls `StateProcessor::apply_prompt_send_error`, which applies the run error `Failed to send prompt: {error}`.
- The text is restored into the prompter. When the buffer already holds new text, the restored text is prepended, followed by a newline.

Typing stays enabled during a run. Only submitting is blocked.

## Wrap and Boundary Rules

- Wrapping is grapheme-based and lossless. There is no word-boundary preference, and no whitespace changes.
- A row is broken when `col + width > viewport` and `col > 0`.
- Each logical line yields at least one row, including empty lines.
- A width of 0 is treated as 1 by `wrap_rows`. The `PrompterState` effective width uses `u16::MAX` when `viewport_width` is 0, which means no soft wrap.
- A cursor at a soft wrap boundary belongs to the next row, except at the end of the text.
- An offset on a soft-wrapped row never lands on the row end. It stays inside that row.
- `ensure_cursor_visible` keeps the cursor row inside the window and clamps `scroll_y` to `rows.len() - PROMPTER_MAX_ROWS`.

## View Rules

- The first text row starts with the marker `> `. Continuation rows are indented by 2 spaces (`PROMPTER_MARKER_WIDTH`).
- The prompt height is 1 to 3 text rows, plus the top and bottom border rows.
- The selection uses `STL_INPUT_SELECTION`, which is reverse video.
- The cursor is shown whenever the prompt area has room, including during a run.
- The prompt block has no title. The help ` Enter to send  |  Ctrl-j newline  |  /q to quit ` is shown right aligned in the status bar, and hidden when the terminal is too narrow.
- Footer: `[Enter] Send`, `[\ Enter] Newline`, `[/q] Quit`, and `[Ctrl-c] Quit`.

## Clipboard

- `zc_common::clipboard::Clipboard` wraps `arboard` with `default-features = false`, so only text is supported.
- One lazily created handle is held by `TuiState`. On Linux, the process that sets the clipboard serves its content, so the handle must stay alive.
- Copy reads from the prompter buffer, never from rendered rows.
- Copy and paste errors, and the `Nothing to copy` case, are reported in the status line.

## Terminal Setup and Bracketed Paste

- `TermSetup::init()` calls `ratatui::init()`, enables mouse capture and bracketed paste, enables keyboard enhancement when supported, and clears the terminal.
- Keyboard enhancement (`DISAMBIGUATE_ESCAPE_CODES`, `REPORT_ALL_KEYS_AS_ESCAPE_CODES`, and `REPORT_ALTERNATE_KEYS`) is pushed whenever `supports_keyboard_enhancement()` confirms it, on any terminal, or when the environment shows Kitty or Alacritty, since the query can fail on them. Under tmux, the flags are never pushed, because tmux does not forward the protocol. `TMUX` is checked before the `ALACRITTY_*` variables, which tmux inherits.
- The `TermSetup` drop is the teardown. It pops the keyboard enhancement flags when pushed, disables mouse capture and bracketed paste, then calls `ratatui::restore()`.
- The teardown runs on normal exit, on early return, and on panic.
- Bracketed paste means Ctrl-v must paste only once. The terminal sends the paste as an `Event::Paste`, so the key handler must not also insert text on that event.

## Platform Notes

- `\` + Enter works on every terminal.
- Shift + Enter is reported only with keyboard enhancement, so only on terminals that support it, such as Kitty and Alacritty.
- Under tmux, Shift + Enter is not reported, since tmux does not forward the keyboard enhancement protocol. Use Alt + Enter, Ctrl-j, or `\` + Enter instead. To get Alt + Enter from Shift + Return in Alacritty, add a `[[keyboard.bindings]]` entry with `key = "Return"`, `mods = "Shift"`, and `chars = "\u001B\r"`.
- macOS: Option + Enter is reported as Alt + Enter only when the terminal treats Option as Alt, even with keyboard enhancement. Alacritty: `window.option_as_alt = "Both"` (or `"OnlyLeft"`). Kitty: `macos_option_as_alt`. Terminal.app: "Use Option as Meta key". iTerm2: "Esc+".
- Terminal.app sends plain Enter for Shift + Enter. Use Ctrl-j, or map Shift-Return to send ESC then CR, which is read as Alt + Enter.

## Manual Check List

- Enter sends the prompt, and `\` + Enter inserts a newline.
- Shift + Enter and Option + Enter insert a newline on terminals that support keyboard enhancement, such as Kitty and Alacritty.
- Ctrl-v pastes once.
- Alt-c copies, and the copy can be pasted into another app while the TUI runs (Linux).
- Alt word jumps work on the target terminals.
- A panic or an early exit restores mouse capture and bracketed paste.
