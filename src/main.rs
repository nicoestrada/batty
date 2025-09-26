use clap::Parser;
use std::{fs, path::PathBuf};

#[derive(Debug, Parser)]
#[command(
    version,
    about = "Set or read battery charge threshold on ASUS laptops"
)]
struct Cli {
    /// Optional custom path to the charge control file
    #[arg(short, long)]
    path: Option<PathBuf>,

    /// Threshold value (0-100). If not provided, the current threshold is printed.
    #[arg(short, long)]
    value: Option<u8>,
}

fn main() {
    let cli = Cli::parse();

    // Use provided path or default to BAT0
    let path = cli.path.unwrap_or_else(|| {
        PathBuf::from("/sys/class/power_supply/BAT0/charge_control_end_threshold")
    });

    if let Some(value) = cli.value {
        // ---- Write mode ----
        if value > 100 {
            eprintln!("Error: value must be between 0 and 100");
            std::process::exit(1);
        }

        if let Err(e) = fs::write(&path, value.to_string()) {
            eprintln!("Failed to write to {:?}: {}", path, e);
            std::process::exit(1);
        }

        println!("Battery charge threshold set to {}%", value);
    } else {
        // ---- Read mode ----
        match fs::read_to_string(&path) {
            Ok(current) => println!("Current battery threshold: {}%", current.trim()),
            Err(e) => {
                eprintln!("Failed to read from {:?}: {}", path, e);
                std::process::exit(1);
            }
        }
    }
}
