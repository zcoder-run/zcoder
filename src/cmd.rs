use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(version, about = "A simple CLI example")]
pub struct CliCmd {
	#[command(subcommand)]
	pub command: Option<SubCmd>,

	/// Start the base server
	#[arg(long)]
	pub base: bool,

	/// The optional prompt to process (if not provided, enters interactive loop)
	pub prompt: Option<String>,

	/// Optional directory path
	#[arg(short, long)]
	pub dir: Option<String>,
}

#[derive(Subcommand, Debug, Clone)]
pub enum SubCmd {
	/// Start the base server
	Base,
}

// region:    --- Tests

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_cmd_base_subcommand() {
		let cmd = CliCmd::parse_from(["zc", "base"]);
		assert!(matches!(cmd.command, Some(SubCmd::Base)));
		assert!(!cmd.base);
	}

	#[test]
	fn test_cmd_base_flag() {
		let cmd = CliCmd::parse_from(["zc", "--base"]);
		assert!(cmd.base);
	}

	#[test]
	fn test_cmd_default_client() {
		let cmd = CliCmd::parse_from(["zc", "hello world", "--dir", "/tmp"]);
		assert!(!cmd.base);
		assert!(cmd.command.is_none());
		assert_eq!(cmd.prompt.as_deref(), Some("hello world"));
		assert_eq!(cmd.dir.as_deref(), Some("/tmp"));
	}
}

// endregion: --- Tests
