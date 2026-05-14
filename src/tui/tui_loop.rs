use crate::exec::{ExecActionEvent, ExecStatusEvent, ExecutorTx};
use crate::tui::{AppActionEvent, AppEvent, AppState, view};
use crate::{Error, Result};
use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::DefaultTerminal;
use tokio::sync::mpsc::{Receiver, Sender};

pub async fn run_ui_loop(
	mut terminal: DefaultTerminal,
	mut app_rx: Receiver<AppEvent>,
	app_tx: Sender<AppEvent>,
	executor_tx: ExecutorTx,
	initial_prompt: Option<String>,
) -> Result<()> {
	let mut state = AppState::new(initial_prompt);

	loop {
		terminal.draw(|f| view::render(f, &state))?;

		let app_event = app_rx.recv().await.ok_or_else(|| Error::custom("App event channel closed"))?;

		match app_event {
			AppEvent::Term(term_event) => {
				if let Event::Key(key) = term_event {
					if key.kind == KeyEventKind::Press {
						match key.code {
							// -- Global Quit
							KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
								let _ = app_tx.send(AppEvent::Action(AppActionEvent::Quit)).await;
							}

							// -- Action
							KeyCode::Enter => {
								let trimmed_input = state.input().trim().to_string();
								if trimmed_input == "/q" {
									let _ = app_tx.send(AppEvent::Action(AppActionEvent::Quit)).await;
								} else if !trimmed_input.is_empty() && !state.is_waiting() {
									let prompt = state.input().to_string();
									let _ = app_tx.send(AppEvent::Action(AppActionEvent::RunPrompt(prompt))).await;
								}
							}

							// -- Input
							KeyCode::Backspace => {
								state.pop_input();
							}
							KeyCode::Char(c) => {
								state.push_input(c);
							}
							_ => {}
						}
					}
				}
			}

			AppEvent::Action(action) => match action {
				AppActionEvent::Quit => break,
				AppActionEvent::RunPrompt(prompt) => {
					state.clear_input();
					state.set_waiting(true);
					state.set_last_error(None);
					executor_tx.send(ExecActionEvent::RunPrompt(prompt)).await?;
				}
			},

			AppEvent::Exec(status) => match status {
				ExecStatusEvent::RunStart => {
					state.set_status("Sending to AI...".to_string());
				}
				ExecStatusEvent::RunEnd => {
					state.set_waiting(false);
					state.set_status("Idle".to_string());
				}
				ExecStatusEvent::RunResult(answer) => {
					state.set_last_answer(Some(answer));
				}
				ExecStatusEvent::RunError(err) => {
					state.set_last_error(Some(err));
				}
			},

			AppEvent::DoRedraw => (),
		}
	}

	Ok(())
}
