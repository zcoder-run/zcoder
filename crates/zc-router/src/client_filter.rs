use crate::msg::RouterMsg;
use zc_core::model::Id;

// region:    --- Client Filter

/// Decides whether a base-originated message reaches the client that owns `client_wks_id`.
///
/// This is the single home for fan-out policy. Later policies (per-run
/// subscriptions, per-entity subscriptions, mute flags) land here instead of
/// being spread through the accept loop.
///
/// A message whose `wks_id` matches the client's is delivered. A message whose
/// `wks_id` is the default is treated as unscoped and delivered; unscoped
/// messages now only cover messages the base intentionally broadcasts to all clients.
/// Any other id belongs to another workspace, so the message is not delivered.
pub fn client_filter(client_wks_id: Id, msg: &RouterMsg) -> bool {
	msg.wks_id == client_wks_id || msg.wks_id == Id::default()
}

// endregion: --- Client Filter

// region:    --- Tests

#[cfg(test)]
mod tests {
	type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>;

	use super::*;
	use crate::exec::ExecCmd;
	use crate::msg::RouterMsgData;
	use zc_common::MsgId;

	fn make_msg(wks_id: Id) -> RouterMsg {
		RouterMsg {
			msg_id: MsgId::new(1),
			wks_id,
			data: RouterMsgData::Exec(ExecCmd::RunPrompt("hello".to_string())),
		}
	}

	#[test]
	fn test_client_filter_matching_id_passes() -> Result<()> {
		// -- Setup & Fixtures
		let client_wks_id = Id::try_from("00000000-0000-0000-0000-000000000001".to_string())?;
		let msg = make_msg(client_wks_id.clone());

		// -- Exec
		let pass = client_filter(client_wks_id, &msg);

		// -- Check
		assert!(pass);

		Ok(())
	}

	#[test]
	fn test_client_filter_other_id_is_dropped() -> Result<()> {
		// -- Setup & Fixtures
		let client_wks_id = Id::try_from("00000000-0000-0000-0000-000000000001".to_string())?;
		let other_wks_id = Id::try_from("00000000-0000-0000-0000-000000000002".to_string())?;
		let msg = make_msg(other_wks_id);

		// -- Exec
		let pass = client_filter(client_wks_id, &msg);

		// -- Check
		assert!(!pass);

		Ok(())
	}

	#[test]
	fn test_client_filter_unscoped_passes() -> Result<()> {
		// -- Setup & Fixtures
		let client_wks_id = Id::try_from("00000000-0000-0000-0000-000000000001".to_string())?;
		let msg = make_msg(Id::default());

		// -- Exec
		let pass = client_filter(client_wks_id, &msg);

		// -- Check
		assert!(pass);

		Ok(())
	}
}

// endregion: --- Tests
