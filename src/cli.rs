use clap::Parser;
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(
    version,
    about = "Set or read battery charge threshold on ASUS laptops"
)]
pub struct Cli {
    #[arg(short, long)]
    pub path: Option<PathBuf>,

    #[arg(short, long)]
    pub value: Option<u8>,

    #[arg(
        short = 'k',
        long,
        default_value = "end",
        help = "Which threshold kind to set (start or end)"
    )]
    pub kind: String,

    #[arg(long, help = "Launch the interactive terminal UI")]
    pub tui: bool,
}
