mod battery;
mod cli;
mod thresholds;
mod tui;

use battery::find_batteries;
use clap::Parser;
use cli::Cli;
use std::path::PathBuf;
use thresholds::{ThresholdKind, Thresholds};

fn main() {
    let cli = Cli::parse();

    let power_supply_path = cli
        .path
        .unwrap_or_else(|| PathBuf::from("/sys/class/power_supply"));

    let bat_paths = find_batteries(&power_supply_path);

    if bat_paths.is_empty() {
        eprintln!("Error: No batteries found in {}", power_supply_path.display());
        eprintln!("Make sure you're running on a laptop with battery support.");
        std::process::exit(1);
    }

    if cli.tui {
        if cli.value.is_some() {
            eprintln!("Error: --value cannot be used with --tui");
            std::process::exit(1);
        }

        if let Err(err) = tui::run_tui(bat_paths) {
            eprintln!("Failed to run TUI: {}", err);
            std::process::exit(1);
        }

        return;
    }

    // Use the first battery for CLI operations
    let battery_path = &bat_paths[0];

    if let Some(value) = cli.value {
        let kind = match cli.kind.to_lowercase().as_str() {
            "start" => ThresholdKind::Start,
            "end" => ThresholdKind::End,
            _ => {
                eprintln!("Error: kind must be either 'start' or 'end'");
                std::process::exit(1);
            }
        };

        let mut thresholds = match Thresholds::load(battery_path) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("Failed to load current thresholds: {}", e);
                std::process::exit(1);
            }
        };

        if let Err(e) = thresholds.set(kind, value) {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }

        if let Err(e) = thresholds.save(battery_path) {
            eprintln!("Failed to save thresholds: {}", e);
            std::process::exit(1);
        }

        println!("Battery charge {} threshold set to {}%", kind, value);
    } else {
        match Thresholds::load(battery_path) {
            Ok(thresholds) => {
                println!("Current battery thresholds:");
                println!("  Start: {}%", thresholds.start);
                println!("  End:   {}%", thresholds.end);
            }
            Err(e) => {
                eprintln!("Failed to read thresholds: {}", e);
                std::process::exit(1);
            }
        }
    }
}
