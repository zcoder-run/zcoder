use crate::Result;

/// Text-only clipboard wrapper.
///
/// The arboard handle is kept and reused, because on Linux the process that
/// set the clipboard content must stay alive to serve it.
#[derive(Default)]
pub struct Clipboard {
	inner: Option<arboard::Clipboard>,
}

impl Clipboard {
	pub fn set_text(&mut self, text: impl Into<String>) -> Result<()> {
		self.inner_mut()?.set_text(text.into())?;
		Ok(())
	}

	pub fn get_text(&mut self) -> Result<String> {
		let text = self.inner_mut()?.get_text()?;
		Ok(text)
	}
}

// region:    --- Support

impl Clipboard {
	fn inner_mut(&mut self) -> Result<&mut arboard::Clipboard> {
		if self.inner.is_none() {
			self.inner = Some(arboard::Clipboard::new()?);
		}
		let handle = self.inner.as_mut().ok_or("clipboard handle not initialized")?;
		Ok(handle)
	}
}

// endregion: --- Support
