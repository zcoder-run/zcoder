use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(version, about = "A simple CLI example")]
pub struct CliCmd {
	#[command(subcommand)]
	pub command: Option<SubCmd>,

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

