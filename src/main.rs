use clap::Parser;
use std::{fs, path::PathBuf};

#[derive(Debug, Parser)]
#[command(
    version,
    about = "Set or read battery charge threshold on ASUS laptops"
)]
struct Cli {
    #[arg(short, long)]
    path: Option<PathBuf>,

    #[arg(short, long)]
    value: Option<u8>,
}

fn main() {
    let cli = Cli::parse();

    let path = cli.path.unwrap_or_else(|| {
        PathBuf::from("/sys/class/power_supply/BAT0/charge_control_end_threshold")
    });

    if let Some(value) = cli.value {
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
        match fs::read_to_string(&path) {
            Ok(current) => println!("Current battery threshold: {}%", current.trim()),
            Err(e) => {
                eprintln!("Failed to read from {:?}: {}", path, e);
                std::process::exit(1);
            }
        }
    }
}
