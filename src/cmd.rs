use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about = "A simple CLI example")]
pub struct CliCmd {
    /// The optional prompt to process (if not provided, enters interactive loop)
    pub prompt: Option<String>,

    /// Optional directory path
    #[arg(short, long)]
    pub dir: Option<String>,
}
